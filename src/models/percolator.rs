use std::num::NonZeroUsize;
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
fn clause_to_mi(c: &Clause) -> MatchItem {
    let lits = c.literals().iter();

    // If ANY of the litteral is negated, we need to return a match all.
    // This is because in this case, we cannot use the positive litterals
    // to get the query candidates. As there might be candidates that have
    // the negated litterals satisfied.
    if lits.clone().any(|l| l.is_negated()) {
        return MatchItem::match_all().with_must_filter();
    }

    let mi = MatchItem::new(lits.clone().fold(Document::default(), |a, l| {
        let pfv = l.percolate_doc_field_value();
        a.with_value(pfv.0, pfv.1)
    }));

    // Add the preheaters from the literals
    lits.fold(mi, |mi, li| {
        if let Some(ph) = li.preheater() {
            mi.with_preheater(ph)
        } else {
            mi
        }
    })
}

/*
    From a CNFQuery, The documents that are meant to be indexed in the percolator
*/
fn cnf_to_matchitems(q: &Query) -> impl Iterator<Item = MatchItem> + use<'_> {
    q.clauses().iter().map(clause_to_mi)
}

#[derive(Debug, Default)]
struct ClauseMatchers {
    positive_index: Index,
}

#[derive(Debug)]
pub struct PercolatorStats {
    n_queries: usize,
    n_preheaters: usize,
    clauses_per_query: Hstats<f64>,
    preheaters_per_query: Hstats<f64>,
}

impl Default for PercolatorStats {
    fn default() -> Self {
        Self {
            n_queries: Default::default(),
            n_preheaters: Default::default(),

            clauses_per_query: Hstats::new(0.0, 50.0, 25),
            preheaters_per_query: Hstats::new(0.0, 50.0, 25),
        }
    }
}

impl std::fmt::Display for PercolatorStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "🔎 N queries={}\n🔥 Preheaters={}\n❓ Clauses per query:\n{}🔥 Perheaters per query:\n{}",
            self.n_queries, self.n_preheaters, self.clauses_per_query, self.preheaters_per_query,
        )
    }
}

/// A builder should you want to build a percolator
/// with different parameters
pub struct PercBuilder {
    n_clauses: NonZeroUsize,
}

impl std::default::Default for PercBuilder {
    fn default() -> Self {
        Self {
            n_clauses: NonZeroUsize::new(3).unwrap(),
        }
    }
}

impl PercBuilder {
    pub fn build(self) -> Percolator {
        Percolator {
            cnf_queries: Vec::new(),
            preheaters: Vec::new(),
            clause_matchers: (0..self.n_clauses.get())
                .map(|_| ClauseMatchers::default())
                .collect(),
            must_filter: RoaringBitmap::new(),
            stats: Default::default(),
        }
    }

    /// Sets the expected number of clauses of indexed queries
    /// to the given value. This help minimizing the number of post-match
    /// checks the percolator has to do.
    ///
    /// You'll have to experiment and benchmark to find
    /// the sweet spot for your specific use case.
    ///
    /// The default is 3.
    ///
    /// Example:
    /// ```
    /// use mokaccino::prelude::*;
    /// use std::num::NonZeroUsize;
    ///
    /// let p = Percolator::builder().n_clauses(NonZeroUsize::new(5).unwrap()).build();
    ///
    /// ```
    pub fn n_clauses(mut self, n: NonZeroUsize) -> Self {
        self.n_clauses = n;
        self
    }
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
pub struct Percolator {
    cnf_queries: Vec<Query>,
    clause_matchers: Vec<ClauseMatchers>,
    // To preheat the document clauses.
    preheaters: Vec<PreHeater>,
    // Holds which queries MUST be finally filtered with
    // their match(document) method.
    must_filter: RoaringBitmap,
    stats: PercolatorStats,
}

impl std::default::Default for Percolator {
    fn default() -> Self {
        Self {
            cnf_queries: Vec::new(),
            preheaters: Vec::new(),
            clause_matchers: (0..3).map(|_| ClauseMatchers::default()).collect(),
            must_filter: RoaringBitmap::new(),
            stats: Default::default(),
        }
    }
}
impl fmt::Display for Percolator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MultiPerc-{}Qs/{}IDXs",
            self.cnf_queries.len(),
            self.clause_matchers.len()
        )
    }
}

impl Percolator {
    // Does this have this preheater yet?
    fn has_preheater(&self, ph: &PreHeater) -> bool {
        self.preheaters.iter().any(|eph| eph.id == ph.id)
    }

    /// Returns a percolator builder for configurability
    /// Example:
    /// ```
    /// use mokaccino::prelude::*;
    ///
    /// let mut p = Percolator::builder().build();
    ///
    /// ```
    pub fn builder() -> PercBuilder {
        PercBuilder::default()
    }

    pub fn stats(&self) -> &PercolatorStats {
        &self.stats
    }

    ///
    /// Adds a query to this percolator. Will panic if
    /// there is more than u32::MAX queries.
    ///
    pub fn add_query(&mut self, q: Query) -> Qid {
        // Get the document from the query
        // and index in the query index
        // The Clause index is controlling the zip.
        let expected_index_len = self.cnf_queries.len() + 1;

        let new_doc_id = self.cnf_queries.len().try_into().expect("Too many queries");
        self.stats.n_queries += 1;

        let mis = cnf_to_matchitems(&q).collect_vec();

        self.stats.clauses_per_query.add(
            TryInto::<u32>::try_into(mis.len())
                .expect("Too many match items - More than u32::MAX !?!")
                .into(),
        );

        if mis.len() > self.clause_matchers.len() {
            self.must_filter.insert(new_doc_id);
        }

        let mut n_preheaters: usize = 0;

        // Add the preheaters from the Match items.
        mis.iter()
            .take(self.clause_matchers.len())
            .flat_map(|mi| mi.preheaters.iter())
            .for_each(|ph| {
                n_preheaters += 1;

                if ph.must_filter {
                    self.must_filter.insert(new_doc_id);
                }

                if !self.has_preheater(ph) {
                    self.preheaters.push(ph.clone());
                    self.stats.n_preheaters += 1;
                }
            });

        self.stats.preheaters_per_query.add(
            TryInto::<u32>::try_into(n_preheaters)
                .expect("Too many preheaters - More than u32::MAX !?!")
                .into(),
        );

        //dbg!(self.preheaters.iter().map(|ph| &ph.id).collect_vec());

        let cms = self.clause_matchers.iter_mut();
        cms.zip(mis.into_iter().chain(iter::repeat(MatchItem::match_all())))
            .for_each(|(ms, mi)| {
                if mi.must_filter {
                    self.must_filter.insert(new_doc_id);
                }
                ms.positive_index.index_document(&mi.doc);
                assert_eq!(ms.positive_index.len(), expected_index_len);
            });

        self.cnf_queries.push(q);
        new_doc_id
    }

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
                *acc &= b;
            })
            .expect("This cannot work with zero clause matchers")
    }
}

mod tests_cnf {
    use crate::models::percolator::MatchItem;

    #[allow(dead_code)]
    fn is_match_all(mi: &MatchItem) -> bool {
        mi.doc.is_match_all()
    }

    #[test]
    fn test_empty() {
        use super::*;
        let cnf = Query::default();
        assert!(cnf_to_matchitems(&cnf).next().is_none());
    }

    #[test]
    fn test_or_with_neg() {
        use super::*;
        use crate::prelude::CNFQueryable;

        let q = !"f1".has_value("v1") | "f2".has_value("v2");
        let mis = cnf_to_matchitems(&q).next().unwrap();
        assert!(is_match_all(&mis));
        assert!(mis.must_filter);
    }

    #[test]
    fn test_literal() {
        use super::*;
        use crate::prelude::CNFQueryable;
        let term_query = TermQuery::new("field", "value");
        let cnf_query = Query::from_termquery(term_query);
        let mi = cnf_to_matchitems(&cnf_query).next().unwrap();
        assert_eq!(mi.doc, Document::default().with_value("field", "value"));

        let cnf_query = !"field".has_value("value");
        let mi = cnf_to_matchitems(&cnf_query).next().unwrap();
        assert!(is_match_all(&mi));
        assert!(mi.must_filter);
    }

    #[test]
    fn test_from_and() {
        use super::*;
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
        let mut mis = cnf_to_matchitems(&combined);
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

        let combined = "Y".has_value("y") | "X".has_value("x");

        let mut mis = cnf_to_matchitems(&combined);
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
        let mut mis = cnf_to_matchitems(&q);
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
}

mod test_extensive;
