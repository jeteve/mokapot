use std::{fmt, iter};

use itertools::Itertools;
use roaring::RoaringBitmap;

use crate::models::{
    cnf::{self, CNFQuery, Clause},
    document::Document,
    index::Index,
    queries::TermQuery,
};

pub type Qid = u32;

fn clause_to_document(c: &Clause, negated: bool) -> Document {
    c.literals()
        .iter()
        .filter(|&l| l.is_negated() ^ !negated)
        .fold(Document::default(), |a, l| {
            a.with_value(l.field(), l.value())
        })
}

/*
    From a CNFQuery, The documents that are meant to be indexed in the percolator
*/
fn cnf_to_documents(q: &CNFQuery, negated: bool) -> impl Iterator<Item = Document> + use<'_> {
    q.clauses()
        .iter()
        .map(move |c| clause_to_document(c, negated))
        .chain(iter::repeat_n(
            if negated {
                Document::default()
            } else {
                Document::match_all()
            },
            1000,
        ))
}

#[derive(Debug, Default)]
struct ClauseMatchers {
    positive_index: Index,
    negative_index: Index,
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
            .map(|ms| dclause.docs_from_idx(&ms.positive_index))
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
            .zip(cnf_to_documents(&q, false))
            .zip(cnf_to_documents(&q, true))
            .for_each(|((ms, pos_doc), neg_doc)| {
                //rintln!("For CNF={} -IDXDOC- {:?}", cnf, doc);
                ms.positive_index.index_document(&pos_doc);
                ms.negative_index.index_document(&neg_doc);
                assert_eq!(ms.positive_index.len(), expected_index_len);
                assert_eq!(ms.negative_index.len(), expected_index_len);
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
            cnf_to_documents(&cnf, false).next(),
            Some(Document::match_all())
        );

        let cnf = CNFQuery::default();
        assert_eq!(
            cnf_to_documents(&cnf, true).next(),
            Some(Document::default())
        );
    }

    #[test]
    fn test_literal() {
        use super::*;
        use crate::prelude::CNFQueryable;
        let term_query = TermQuery::new("field", "value");
        let cnf_query = CNFQuery::from_termquery(term_query);
        let mut docs = cnf_to_documents(&cnf_query, false);
        assert_eq!(
            docs.next(),
            Some(Document::default().with_value("field", "value"))
        );

        let cnf_query = !"field".has_value("value");
        let mut docs = cnf_to_documents(&cnf_query, true);
        assert_eq!(
            docs.next(),
            Some(Document::default().with_value("field", "value"))
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
        let mut docs = cnf_to_documents(&combined, false);
        assert_eq!(
            docs.next(),
            Some(Document::default().with_value("field1", "value1"))
        );
        assert_eq!(
            docs.next(),
            Some(Document::default().with_value("field2", "value2"))
        );
        assert_eq!(docs.next(), Some(Document::match_all()));
    }

    #[test]
    fn test_from_or() {
        use super::super::cnf::CNFQueryable;
        use super::*;

        let combined = "Y".has_value("y") | "X".has_value("x");

        let mut docs = cnf_to_documents(&combined, false);
        assert_eq!(
            docs.next(),
            Some(
                Document::default()
                    .with_value("X", "x")
                    .with_value("Y", "y")
            )
        );
        assert_eq!(docs.next(), Some(Document::match_all()));

        // (x AND Y) OR Z:
        // The Z
        let q = ("X".has_value("x") & "Y".has_value("y")) | "Z".has_value("z");
        assert_eq!(q.to_string(), "(AND (OR X=x Z=z) (OR Y=y Z=z))");
        let mut docs = cnf_to_documents(&q, false);
        assert_eq!(
            docs.next(),
            Some(
                Document::default()
                    .with_value("X", "x")
                    .with_value("Z", "z")
            )
        );
        assert_eq!(
            docs.next(),
            Some(
                Document::default()
                    .with_value("Y", "y")
                    .with_value("Z", "z")
            )
        );
        assert_eq!(docs.next(), Some(Document::match_all()));
    }
}
