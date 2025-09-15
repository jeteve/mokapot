use std::rc::Rc;
use std::{fmt, iter};

use itertools::Itertools;
use roaring::RoaringBitmap;

use crate::itertools::InPlaceReduce;

use crate::models::index::DocId;
use crate::models::{
    cnf::{CNFQuery, Clause},
    document::Document,
    index::Index,
    queries::TermQuery,
};

pub type Qid = u32;

#[derive(Clone)]
// Extends Clauses comming from percolated document with extra termqueries.
struct ClauseExpander(Rc<dyn Fn(Clause) -> Clause>);
impl std::fmt::Debug for ClauseExpander {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ClauseExpander")
            .field(&"_OPAQUE FUNCTION_")
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PreHeater {
    id: Rc<str>,
    expand_clause: ClauseExpander,
}

#[derive(Clone, Debug)]
struct MatchItem {
    must_filter: bool,
    doc: Document,
    preheaters: Vec<PreHeater>,
}

impl MatchItem {
    fn new(doc: Document) -> Self {
        MatchItem {
            doc,
            must_filter: false,
            preheaters: vec![],
        }
    }

    pub fn with_preheater(mut self, ph: PreHeater) -> Self {
        self.preheaters.push(ph);
        self
    }

    fn is_match_all(&self) -> bool {
        self.doc.is_match_all()
    }

    fn match_all() -> Self {
        Self::new(Document::match_all())
    }

    fn with_must_filter(mut self) -> Self {
        self.must_filter = true;
        self
    }
}

// The docs Ids from the index mathing this clause
// This is only used in the context of percolation,
//    this clause will NOT have any negatives.
//    this clause will NOT have any non-term litterals.
// Migrate to percolator please.
pub fn clause_docs_from_idx(c: &Clause, index: &Index) -> RoaringBitmap {
    let mut ret = RoaringBitmap::new();
    c.literals()
        .iter()
        .map(|l| l.percolate_docs_from_idx(index))
        .for_each(|bm| ret |= bm);

    ret
}

// The docs Ids from the index matching this clause
// In the context of a percolation only.
pub fn clause_docs_from_idx_iter<'a>(
    c: &Clause,
    index: &'a Index,
) -> impl Iterator<Item = DocId> + use<'a> {
    clause_docs_from_idx(c, index).into_iter()
}

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
fn cnf_to_matchitems(q: &CNFQuery) -> impl Iterator<Item = MatchItem> + use<'_> {
    q.clauses().iter().map(clause_to_mi)
}

#[derive(Debug, Default)]
struct ClauseMatchers {
    positive_index: Index,
}

#[derive(Debug)]
pub struct Percolator {
    cnf_queries: Vec<CNFQuery>,
    clause_matchers: Vec<ClauseMatchers>,
    // To preheat the document clauses.
    preheaters: Vec<PreHeater>,
    // Holds which queries MUST be finally filtered with
    // their match(document) method.
    must_filter: RoaringBitmap,
}

impl std::default::Default for Percolator {
    fn default() -> Self {
        Self {
            cnf_queries: Vec::new(),
            preheaters: Vec::new(),
            clause_matchers: (0..3).map(|_| ClauseMatchers::default()).collect(),
            must_filter: RoaringBitmap::new(),
        }
    }
}

pub struct TrackedQid {
    pre_idx: usize,
    post_idx: usize,
    pub qid: Qid,
}
impl TrackedQid {
    pub fn n_skipped(&self) -> usize {
        self.pre_idx - self.post_idx
    }
}

impl Percolator {
    /// Percolate a document through this, returning an iterator
    /// of the matching query IDs
    ///
    /// The `doc_like` argument can be a `&Document` or any type that implements
    /// the `ToCowDocument` trait, like an array of key-value tuples.
    pub fn percolate<'b>(&self, d: &'b Document) -> impl Iterator<Item = Qid> + use<'b, '_> {
        self.bs_from_document(d).into_iter().filter(move |&qid| {
            !self.must_filter.contains(qid) || self.cnf_queries[qid as usize].matches(d)
        })
    }

    fn bs_from_document(&self, d: &Document) -> RoaringBitmap {
        // This is where the magic happens.
        let mut dclause = d.to_clause();
        // Add the match all to match all queries
        dclause.add_termquery(TermQuery::match_all());

        // Preheat the clause
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

    ///
    /// Adds a query to this percolator. Will panic if
    /// there is more than u32::MAX queries.
    ///
    pub fn add_query(&mut self, q: CNFQuery) -> Qid {
        // Get the document from the query
        // and index in the query index
        // The Clause index is controlling the zip.
        let expected_index_len = self.cnf_queries.len() + 1;

        let new_doc_id = self.cnf_queries.len().try_into().expect("Too many queries");

        let mis = cnf_to_matchitems(&q).collect_vec();

        if mis.len() > self.clause_matchers.len() {
            self.must_filter.insert(new_doc_id);
        }

        // Add the preheaters from the Match items.
        mis.iter()
            .flat_map(|mi| mi.preheaters.iter())
            .for_each(|ph| {
                if !self.has_preheater(ph) {
                    self.preheaters.push(ph.clone())
                }
            });

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

    pub fn get_query(&self, qid: Qid) -> &CNFQuery {
        &self.cnf_queries[qid as usize]
    }
}

mod test_cnf {

    #[test]
    fn test_empty() {
        use super::*;
        let cnf = CNFQuery::default();
        assert!(cnf_to_matchitems(&cnf).next().is_none());
    }

    #[test]
    fn test_or_with_neg() {
        use super::*;
        use crate::prelude::CNFQueryable;

        let q = !"f1".has_value("v1") | "f2".has_value("v2");
        let mis = cnf_to_matchitems(&q).next().unwrap();
        assert!(mis.is_match_all());
        assert!(mis.must_filter);
    }

    #[test]
    fn test_literal() {
        use super::*;
        use crate::prelude::CNFQueryable;
        let term_query = TermQuery::new("field", "value");
        let cnf_query = CNFQuery::from_termquery(term_query);
        let mi = cnf_to_matchitems(&cnf_query).next().unwrap();
        assert_eq!(mi.doc, Document::default().with_value("field", "value"));

        let cnf_query = !"field".has_value("value");
        let mi = cnf_to_matchitems(&cnf_query).next().unwrap();
        assert!(mi.is_match_all());
        assert!(mi.must_filter);
    }

    #[test]
    fn test_from_and() {
        use super::*;
        let term_query1 = TermQuery::new("field1", "value1");
        let term_query2 = TermQuery::new("field2", "value2");
        let cnf_query1 = CNFQuery::from_termquery(term_query1);
        let cnf_query2 = CNFQuery::from_termquery(term_query2);
        let combined = CNFQuery::from_and(vec![cnf_query1, cnf_query2]);
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
