use std::num::{NonZeroUsize, TryFromIntError};
use std::{fmt, iter};

use hstats::Hstats;
use itertools::Itertools;
use roaring::RoaringBitmap;

use crate::itertools::InPlaceReduce;

use crate::models::{
    cnf::{Clause, Query},
    document::Document,
    index::Index,
    queries::term::TermQuery,
};

pub(crate) mod tools;
use tools::*;

pub type Qid = u32;

// The docs Ids from the index mathing this clause
// This is only used in the context of percolation,
//    this clause will NOT have any negatives.
//    this clause will NOT have any non-term litterals.
// Migrate to percolator please.
pub(crate) fn clause_docs_from_idx(c: &Clause, index: &Index) -> RoaringBitmap {
    let mut ret = RoaringBitmap::new();
    c.literals()
        .iter()
        .map(|l| l.percolate_docs_from_idx(index))
        .for_each(|bm| ret |= bm);

    ret
}

// For indexing clauses.
fn clause_to_mi(c: &Clause, conf: &PercolatorConfig) -> MatchItem {
    let lits = c.literals().iter();

    // If ANY of the litteral is negated, we need to return a match all.
    // This is because in this case, we cannot use the positive litterals
    // to get the query candidates. As there might be candidates that have
    // the negated litterals satisfied.
    if lits.clone().any(|l| l.is_negated()) {
        return MatchItem::match_all().with_must_filter();
    }

    let mi = MatchItem::new(
        lits.clone().fold(Document::default(), |a, l| {
            let pfvs = l.percolate_doc_field_values(conf);
            pfvs.into_iter()
                .fold(a, |a, pfv| a.with_value(pfv.0, pfv.1))
        }),
        c.cost(),
    );

    // Add the preheaters from the literals
    lits.fold(mi, |mi, li| {
        if let Some(ph) = li.preheater(conf) {
            mi.with_preheater(ph)
        } else {
            mi
        }
    })
}

/*
    From a CNFQuery, The documents that are meant to be indexed in the percolator
    In order of costs. Cheaper ones first.
*/
fn cnf_to_matchitems(q: &Query, conf: &PercolatorConfig) -> impl Iterator<Item = MatchItem> {
    q.clauses()
        .iter()
        .map(|c| clause_to_mi(c, conf))
        .sorted_by_key(|mi| mi.cost)
}

// A structure to match just one clause.
#[derive(Debug, Default)]
struct ClauseMatcher {
    positive_index: Index,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PercolatorConfig {
    pub(crate) n_clause_matchers: NonZeroUsize,
    pub(crate) prefix_sizes: Vec<usize>,
}

impl Default for PercolatorConfig {
    fn default() -> Self {
        Self {
            n_clause_matchers: NonZeroUsize::new(3).unwrap(),
            prefix_sizes: vec![2, 10, 100, 1000, 2000],
        }
    }
}
impl PercolatorConfig {
    /// The number of clause matchers to use.
    /// More clause matchers means less queries
    /// will need to be fully matched with their
    /// `matches(document)` method.
    ///
    /// The default is 3.
    pub fn n_clause_matchers(&self) -> NonZeroUsize {
        self.n_clause_matchers
    }

    /// The allowed prefix sizes for prefix queries.
    /// This is used to create synthetic fields for
    /// indexing the prefixes.
    ///
    /// The default is `[2, 10, 100, 1000, 2000]`
    pub fn prefix_sizes(&self) -> &[usize] {
        &self.prefix_sizes
    }
}

///
/// Some statistics about the percolator
/// to help adapting the configuration to the
/// reality of the query corpus.
/// [`Display`] is implemented for quick convenient output.
#[derive(Debug)]
pub struct PercolatorStats {
    n_queries: usize,
    n_preheaters: usize,
    clauses_per_query: Hstats<f64>,
    preheaters_per_query: Hstats<f64>,
    prefix_lengths: Hstats<f64>,
}

impl Default for PercolatorStats {
    fn default() -> Self {
        let proto_hstat = Hstats::new(0.0, 50.0, 25);

        Self {
            n_queries: Default::default(),
            n_preheaters: Default::default(),

            clauses_per_query: proto_hstat.clone(),
            preheaters_per_query: proto_hstat.clone(),
            prefix_lengths: proto_hstat.clone(),
        }
    }
}

impl std::fmt::Display for PercolatorStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "🔎 N queries={}
🔥 Preheaters={}
❓ Clauses per query:
{}
🔥 Preheaters per query:
{}
📏 Prefix lengths:
{}",
            self.n_queries,
            self.n_preheaters,
            self.clauses_per_query,
            self.preheaters_per_query,
            self.prefix_lengths,
        )
    }
}

impl PercolatorStats {
    /// The number of queries considered
    pub fn n_queries(&self) -> usize {
        self.n_queries
    }

    /// The number of distinct pre heating functions
    /// coming from indexed queries for the percolator.
    pub fn n_preheaters(&self) -> usize {
        self.n_preheaters
    }

    /// Distribution of number of clauses per query
    pub fn clauses_per_query(&self) -> &Hstats<f64> {
        &self.clauses_per_query
    }

    /// Distribution of number of pre heating function per query
    pub fn preheaters_per_query(&self) -> &Hstats<f64> {
        &self.preheaters_per_query
    }
}

#[derive(Debug)]
pub enum PercolatorError {
    /// Too many queries added to the percolator (more than u32::MAX)
    TooManyQueries,
    /// A prefix in a prefix query is too long (length exceeds u32::MAX)
    PrefixTooLong(usize),
    /// A query has too many clauses (exceeds u32::MAX)
    TooManyClauses,
    /// A query has too many non pure-term query atoms (exceeds u32::MAX)
    TooManyPreheaters,
}

/// This is the primary object you need to keep to percolate documents
/// through a set of queries.
///
/// Example:
/// ```
/// use mokaccino::prelude::*;
///
/// let mut p = Percolator::default();
/// let qid = p.add_query("field".has_value("value"));
/// assert_eq!(p.percolate(&[("field", "value")].into()).next().unwrap(), qid);
/// ```
///
/// See more examples in the top level documentation.
///
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PercolatorCore {
    // Serialisable data.
    config: PercolatorConfig,
    cnf_queries: Vec<Query>,
    unindexed_qids: RoaringBitmap,

    // Only when the serde feature is on, add the serde(skip) attribute
    // so this does not get serialised.
    #[cfg_attr(feature = "serde", serde(skip))]
    // Operational stuff. Not serialisable.
    clause_matchers: Vec<ClauseMatcher>,
    // To preheat the document clauses.
    #[cfg_attr(feature = "serde", serde(skip))]
    preheaters: Vec<PreHeater>,
    // Holds which queries MUST be finally filtered with
    // their match(document) method.
    #[cfg_attr(feature = "serde", serde(skip))]
    must_filter: RoaringBitmap,
    #[cfg_attr(feature = "serde", serde(skip))]
    stats: PercolatorStats,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for PercolatorCore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Helper is a stripped down version of the percolator
        // with only the fields that are serialised/deserialised.
        #[derive(serde::Deserialize)]
        struct Helper {
            config: PercolatorConfig,
            cnf_queries: Vec<Query>,
            unindexed_qids: RoaringBitmap,
        }

        let helper = Helper::deserialize(deserializer)?;
        let mut p = PercolatorCore::from_config(helper.config);

        // Rebuild the indexes from the queries.
        for q in helper.cnf_queries {
            p.add_query(q);
        }

        // and from the removed queries.
        for qid in helper.unindexed_qids {
            p.remove_qid(qid);
        }

        Ok(p)
    }
}

impl std::default::Default for PercolatorCore {
    fn default() -> Self {
        let default_config = PercolatorConfig::default();
        PercolatorCore::from_config(default_config)
    }
}
impl fmt::Display for PercolatorCore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MultiPerc-{}Qs/{}IDXs",
            self.cnf_queries.len(),
            self.clause_matchers.len()
        )
    }
}

fn usize_to_f64(u: usize) -> Result<f64, TryFromIntError> {
    let u: u32 = u.try_into()?;
    Ok(f64::from(u))
}

impl PercolatorCore {
    pub fn from_config(config: PercolatorConfig) -> Self {
        Self {
            cnf_queries: Vec::new(),
            unindexed_qids: RoaringBitmap::new(),

            preheaters: Vec::new(),
            clause_matchers: (0..config.n_clause_matchers().get())
                .map(|_| ClauseMatcher::default())
                .collect(),
            must_filter: RoaringBitmap::new(),
            stats: Default::default(),

            config,
        }
    }

    // Does this have this preheater yet?
    fn has_preheater(&self, ph: &PreHeater) -> bool {
        self.preheaters.iter().any(|eph| eph.id == ph.id)
    }

    /// The percolator statistics
    /// Mainly for display.
    pub fn stats(&self) -> &PercolatorStats {
        &self.stats
    }

    /// Adds a query to this percolator. Will panic if
    /// there is more than u32::MAX queries.
    /// Example:
    /// ```
    /// use mokaccino::prelude::*;
    /// let mut p = Percolator::default();
    /// let qid = p.add_query("field".has_value("value"));
    /// ```
    ///
    pub fn add_query(&mut self, q: Query) -> Qid {
        self.safe_add_query(q).unwrap()
    }

    /// Safely adds a query to this percolator, reporting errors
    /// when there are too many queries or other limits are exceeded.
    ///
    /// Example:
    /// ```
    /// use mokaccino::prelude::*;
    /// let mut p = Percolator::default();
    /// match p.safe_add_query("field".has_value("value")) {
    ///    Ok(qid) => println!("Added query with id {}", qid),
    ///   Err(e) => println!("Failed to add query: {:?}", e),
    /// }
    /// ```
    ///
    pub fn safe_add_query(&mut self, q: Query) -> Result<Qid, PercolatorError> {
        // Get the document from the query
        // and index in the query index
        // The Clause index is controlling the zip.
        let expected_index_len = self.cnf_queries.len() + 1;

        let new_doc_id = self
            .cnf_queries
            .len()
            .try_into()
            .map_err(|_| PercolatorError::TooManyQueries)?;
        self.stats.n_queries += 1;

        for prefix_query in q.prefix_queries() {
            self.stats.prefix_lengths.add(
                usize_to_f64(prefix_query.prefix().len())
                    .map_err(|_| PercolatorError::PrefixTooLong(prefix_query.prefix().len()))?,
            );
        }

        let mut mis = cnf_to_matchitems(&q, &self.config).collect_vec();

        self.stats
            .clauses_per_query
            .add(usize_to_f64(mis.len()).map_err(|_| PercolatorError::TooManyClauses)?);

        if mis.len() > self.clause_matchers.len() {
            self.must_filter.insert(new_doc_id);
        }

        let mut n_preheaters: usize = 0;

        // Add the preheaters from the Match items.
        // Note that this will EMPTY all the preheaters from
        // the match items. So dont expect to use them later.
        for preheater in mis
            .iter_mut()
            .take(self.clause_matchers.len())
            .flat_map(|mi| std::mem::take(&mut mi.preheaters).into_iter())
        {
            n_preheaters += 1;

            if preheater.must_filter {
                self.must_filter.insert(new_doc_id);
            }

            if !self.has_preheater(&preheater) {
                self.preheaters.push(preheater);
                self.stats.n_preheaters += 1;
            }
        }

        self.stats
            .preheaters_per_query
            .add(usize_to_f64(n_preheaters).map_err(|_| PercolatorError::TooManyPreheaters)?);

        let cms = self.clause_matchers.iter_mut();
        for (clause_matcher, match_item) in
            cms.zip(mis.into_iter().chain(iter::repeat(MatchItem::match_all())))
        {
            if match_item.must_filter {
                self.must_filter.insert(new_doc_id);
            }
            clause_matcher
                .positive_index
                .index_document(&match_item.doc);

            assert_eq!(clause_matcher.positive_index.len(), expected_index_len);
        }

        self.cnf_queries.push(q);
        Ok(new_doc_id)
    }

    /// Removes a query from this percolator by Query ID.
    ///
    /// Returns true if the query was removed, false if
    /// it had already been removed earlier.
    ///
    /// Example:
    /// ```
    /// use mokaccino::prelude::*;
    ///
    /// let mut p = Percolator::default();
    /// let qid = p.add_query("field".has_value("value"));
    /// assert!( p.safe_get_query(qid).is_some() );
    ///
    /// assert!( p.remove_qid(qid) ); // was removed.
    /// assert!( ! p.remove_qid(qid) ); // already removed.
    /// assert!( p.safe_get_query(qid).is_none() );
    /// ```
    pub fn remove_qid(&mut self, qid: Qid) -> bool {
        // mark as unindexed.
        if !self.unindexed_qids.insert(qid) {
            // Value was already marked as unindexed.
            return false;
        }

        for cm in self.clause_matchers.iter_mut() {
            cm.positive_index.unindex_docid(qid);
        }

        // must_filter is now useless.
        self.must_filter.remove(qid);
        true
    }

    /// Safe version of get_query. Will be None if no such query exists.
    pub fn safe_get_query(&self, qid: Qid) -> Option<&Query> {
        if !self.unindexed_qids.contains(qid) {
            self.cnf_queries.get(qid as usize)
        } else {
            None
        }
    }

    /// Get the query identified by the Qid
    pub fn get_query(&self, qid: Qid) -> &Query {
        &self.cnf_queries[qid as usize]
    }

    ///
    /// Percolate a document through this, returning an iterator
    /// of the matching query IDs
    ///
    pub fn percolate<'b>(&self, d: &'b Document) -> impl Iterator<Item = Qid> + use<'b, '_> {
        self.bs_from_document(d).into_iter().filter(move |&qid| {
            !self.must_filter.contains(qid) || self.cnf_queries[qid as usize].matches(d)
        })
    }

    // Get a RoaringBitMap from the document, using the clause matchers.
    fn bs_from_document(&self, d: &Document) -> RoaringBitmap {
        // This is where the magic happens.
        let mut dclause = d.to_clause();
        // Add the match all to match all queries
        dclause.add_termquery(TermQuery::match_all());

        // Preheat the clause
        // This adds a bit of time to the percolation.
        dclause = self
            .preheaters
            .iter()
            .fold(dclause, |c, ph| ph.expand_clause.0(c));

        self.clause_matchers
            .iter()
            .map(|ms| clause_docs_from_idx(&dclause, &ms.positive_index))
            .reduce_inplace(|acc, b| {
                if acc.is_empty() {
                    true // Already empty. Stop the reduction.
                } else {
                    *acc &= b; // Not empty. Process and stop the reduction if now empty
                    acc.is_empty()
                }
            })
            .unwrap_or(RoaringBitmap::new())
    }
}

mod tests_cnf {
    use crate::models::percolator_core::MatchItem;

    #[allow(dead_code)]
    fn is_match_all(mi: &MatchItem) -> bool {
        mi.doc.is_match_all()
    }

    #[test]
    fn test_empty() {
        use super::*;
        let cnf = Query::default();
        let config = PercolatorConfig::default();
        assert!(cnf_to_matchitems(&cnf, &config).next().is_none());
    }

    #[test]
    fn test_or_with_neg() {
        use super::*;
        use crate::prelude::CNFQueryable;

        let q = !"f1".has_value("v1") | "f2".has_value("v2");
        let config = PercolatorConfig::default();
        let mis = cnf_to_matchitems(&q, &config).next().unwrap();
        assert!(is_match_all(&mis));
        assert!(mis.must_filter);
    }

    #[test]
    fn test_literal() {
        use super::*;
        use crate::prelude::CNFQueryable;
        let term_query = TermQuery::new("field", "value");
        let cnf_query = Query::from_termquery(term_query);
        let config = PercolatorConfig::default();
        let mi = cnf_to_matchitems(&cnf_query, &config).next().unwrap();
        assert_eq!(mi.doc, Document::default().with_value("field", "value"));

        let cnf_query = !"field".has_value("value");
        let mi = cnf_to_matchitems(&cnf_query, &config).next().unwrap();
        assert!(is_match_all(&mi));
        assert!(mi.must_filter);
    }

    #[test]
    fn test_from_and() {
        use super::*;
        let config = PercolatorConfig::default();
        let term_query1 = TermQuery::new("field1", "value1");
        let term_query2 = TermQuery::new("field2", "value2");
        let cnf_query1 = Query::from_termquery(term_query1);
        let cnf_query2 = Query::from_termquery(term_query2);
        let combined = Query::from_and(vec![cnf_query1, cnf_query2]);
        // Structure would be:
        assert_eq!(
            combined.to_string(),
            "(AND (OR field1=value1) (OR field2=value2))"
        );
        let mut mis = cnf_to_matchitems(&combined, &config);
        assert_eq!(
            mis.next().unwrap().doc,
            Document::default().with_value("field1", "value1")
        );
        assert_eq!(
            mis.next().unwrap().doc,
            Document::default().with_value("field2", "value2")
        );
        assert!(mis.next().is_none());
    }

    #[test]
    fn test_from_or() {
        use super::super::cnf::CNFQueryable;
        use super::*;

        let config = PercolatorConfig::default();
        let combined = "Y".has_value("y") | "X".has_value("x");

        let mut mis = cnf_to_matchitems(&combined, &config);
        assert_eq!(
            mis.next().unwrap().doc,
            Document::default()
                .with_value("X", "x")
                .with_value("Y", "y")
        );
        assert!(mis.next().is_none());

        // (x AND Y) OR Z:
        // The Z
        let q = ("X".has_value("x") & "Y".has_value("y")) | "Z".has_value("z");
        assert_eq!(q.to_string(), "(AND (OR X=x Z=z) (OR Y=y Z=z))");
        let mut mis = cnf_to_matchitems(&q, &config);
        assert_eq!(
            mis.next().unwrap().doc,
            Document::default()
                .with_value("X", "x")
                .with_value("Z", "z")
        );
        assert_eq!(
            mis.next().unwrap().doc,
            Document::default()
                .with_value("Y", "y")
                .with_value("Z", "z")
        );
        assert!(mis.next().is_none());
    }

    #[test]
    fn test_sorting_by_cost() {
        use super::*;
        use crate::prelude::CNFQueryable;
        use itertools::Itertools;

        // A prefix query is more expensive than a term query.
        let q = "field".has_value("cheap") & "field".has_prefix("expensive");
        let config = PercolatorConfig::default();
        let items = cnf_to_matchitems(&q, &config).collect_vec();
        assert_eq!(items.len(), 2);
        assert!(items[0].cost < items[1].cost);
    }
}

mod test_extensive;
