use std::{fmt, iter};

use itertools::Itertools;
use roaring::RoaringBitmap;

use crate::models::{
    cnf::{CNFQuery, Clause},
    document::Document,
    index::Index,
    queries::TermQuery,
};

pub type Qid = u32;


#[derive(Clone, Debug, PartialEq)]
struct MatchItem{
    doc: Document
}

impl MatchItem{
    fn match_all() -> Self{
        Self{ doc: Document::match_all()}
    }
}


fn clause_to_mi(c: &Clause) -> MatchItem {
    let lits = c.literals().iter();

    // If ANY of the litteral is negated, we need to return a match all.
    // This is because in this case, we cannot use the positive litterals
    // to get the query candidates. As there might be candidates that have
    // the negated litterals satisfied.
    if lits.clone().any(|l| l.is_negated()) {
        return MatchItem::match_all();
    }

    MatchItem{ doc: lits.fold(Document::default(), |a, l| {
        a.with_value(l.field(), l.value())
    }) }
}

/*
    From a CNFQuery, The documents that are meant to be indexed in the percolator
*/
fn cnf_to_matchitems(q: &CNFQuery) -> impl Iterator<Item = MatchItem> + use<'_> {
    q.clauses()
        .iter()
        .map(clause_to_mi)
        .chain(iter::repeat_n(
                MatchItem::match_all()
            ,
            1000,
        ))
}

#[derive(Debug, Default)]
struct ClauseMatchers {
    positive_index: Index,
}

#[derive(Debug)]
pub struct Percolator {
    cnf_queries: Vec<CNFQuery>,
    clause_idxs: Vec<ClauseMatchers>,
}

impl std::default::Default for Percolator {
    fn default() -> Self {
        Self {
            cnf_queries: Vec::new(),
            clause_idxs: (0..3).map(|_| ClauseMatchers::default()).collect(),
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
    pub fn percolate<'b>(&self, d: &'b Document) -> impl Iterator<Item = Qid> + use<'b, '_> {
        self.bs_from_document(d)
            .into_iter()
            .filter(|&qid| self.cnf_queries[qid as usize].matches(d))
    }

    fn bs_from_document(&self, d: &Document) -> RoaringBitmap {
        // This is where the magic happens.
        let mut dclause = d.to_clause();
        // Add the match all to match all queries
        dclause.add_termquery(TermQuery::match_all());

        let mut clause_bss = self
            .clause_idxs
            .iter()
            .map(|ms| 
                // The queries that have been indexed as positive.
                dclause.docs_from_idx(&ms.positive_index)
            )
            .collect_vec();

        clause_bss.reverse();

        // There is at least one index, so at least one clause bitset
        let mut res = clause_bss.pop().unwrap();
        for other_bs in clause_bss.iter().rev() {
            res &= other_bs;
        }

        res
    }
}

impl fmt::Display for Percolator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MultiPerc-{}Qs/{}IDXs",
            self.cnf_queries.len(),
            self.clause_idxs.len()
        )
    }
}

impl Percolator {
    ///
    /// Adds a query to this percolator. Will panic if
    /// there is more than u32::MAX queries.
    ///
    pub fn add_query(&mut self, q: CNFQuery) -> Qid {
        // Get the document from the query
        // and index in the query index
        // The Clause index is controlling the zip.
        let expected_index_len = self.cnf_queries.len() + 1;

        self.clause_idxs
            .iter_mut()
            .zip(cnf_to_matchitems(&q))
            .for_each(|(ms, mi)| {
                ms.positive_index.index_document(&mi.doc);
                assert_eq!(ms.positive_index.len(), expected_index_len);
            });

        let new_doc_id = self.cnf_queries.len();
        self.cnf_queries.push(q);
        new_doc_id.try_into().expect("Too many queries")
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
        assert_eq!(
            cnf_to_matchitems(&cnf).next(),
            Some(MatchItem::match_all())
        );
    }

    #[test]
    fn test_literal() {
        use super::*;
        use crate::prelude::CNFQueryable;
        let term_query = TermQuery::new("field", "value");
        let cnf_query = CNFQuery::from_termquery(term_query);
        let mut docs = cnf_to_matchitems(&cnf_query);
        assert_eq!(
            docs.next(),
            Some(MatchItem{doc: Document::default().with_value("field", "value")})
        );

        let cnf_query = !"field".has_value("value");
        let mut docs = cnf_to_matchitems(&cnf_query);
        assert_eq!(
            docs.next(),
            Some(MatchItem{doc: Document::match_all()})
        );
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
        let mut docs = cnf_to_matchitems(&combined);
        assert_eq!(
            docs.next(),
            Some(MatchItem{doc: Document::default().with_value("field1", "value1")})
        );
        assert_eq!(
            docs.next(),
            Some(MatchItem{doc: Document::default().with_value("field2", "value2")})
        );
        assert_eq!(docs.next(), Some(MatchItem{ doc: Document::match_all()}));
    }

    #[test]
    fn test_from_or() {
        use super::super::cnf::CNFQueryable;
        use super::*;

        let combined = "Y".has_value("y") | "X".has_value("x");

        let mut docs = cnf_to_matchitems(&combined);
        assert_eq!(
            docs.next(),
            Some(MatchItem{ doc:
                Document::default()
                    .with_value("X", "x")
                    .with_value("Y", "y") }
            )
        );
        assert_eq!(docs.next(), Some(MatchItem{doc: Document::match_all()}));

        // (x AND Y) OR Z:
        // The Z
        let q = ("X".has_value("x") & "Y".has_value("y")) | "Z".has_value("z");
        assert_eq!(q.to_string(), "(AND (OR X=x Z=z) (OR Y=y Z=z))");
        let mut docs = cnf_to_matchitems(&q);
        assert_eq!(
            docs.next(),
            Some(MatchItem{doc:
                Document::default()
                    .with_value("X", "x")
                    .with_value("Z", "z")
            })
        );
        assert_eq!(
            docs.next(),
            Some(MatchItem{ doc:
                Document::default()
                    .with_value("Y", "y")
                    .with_value("Z", "z")
            })
        );
        assert_eq!(docs.next(), Some(MatchItem{doc: Document::match_all()}));
    }
}
