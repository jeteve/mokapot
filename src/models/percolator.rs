use std::rc::Rc;
use std::{fmt, iter};

use itertools::Itertools;
use roaring::RoaringBitmap;

use crate::models::{
    cnf::{CNFQuery, Clause},
    documents::Document,
    index::Index,
    iterators::ConjunctionIterator,
    queries::{Query, TermQuery},
};

pub type Qid = u32;

fn clause_to_document(c: &Clause) -> Document {
    c.literals().iter().fold(Document::default(), |a, l| {
        a.with_value(l.field(), l.term())
    })
}

/*
    From a CNFQuery, The documents that are meant to be indexed in the percolator
*/
fn cnf_to_documents(q: &CNFQuery) -> impl Iterator<Item = Document> + use<'_> {
    q.clauses()
        .iter()
        .map(clause_to_document)
        .chain(iter::repeat(Document::match_all()).take(1000))
}

#[derive(Debug)]
pub struct MultiPercolator {
    queries: Vec<Rc<dyn Query>>,
    cnf_queries: Vec<CNFQuery>,
    clause_idxs: Vec<Index>,
}

impl std::default::Default for MultiPercolator {
    fn default() -> Self {
        Self {
            queries: Vec::new(),
            cnf_queries: Vec::new(),
            clause_idxs: (0..3).map(|_| Index::new()).collect(),
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

impl MultiPercolator {
    pub fn bs_qids_from_document<'b>(
        &self,
        d: &'b Document,
    ) -> impl Iterator<Item = Qid> + use<'b, '_> {
        self.bs_from_document(d)
            .into_iter()
            .filter(|&qid| self.cnf_queries[qid as usize].matches(d))
    }

    // This is catastrophic compared to bs_qids_from_document.
    pub fn it_from_document<'a>(&self, d: &'a Document) -> impl Iterator<Item = Qid> + use<'a, '_> {
        let clause_its = self
            .clause_idxs
            .iter()
            .map(|idx| d.to_clause().it_from_idx(idx))
            .collect_vec();
        ConjunctionIterator::new(clause_its)
            .filter(|&qid| self.cnf_queries[qid as usize].matches(d))
    }

    // Clearly not a good idea..
    // DO NOT use this..
    pub fn hybrid_qids_from_document<'b>(
        &self,
        d: &'b Document,
    ) -> impl Iterator<Item = Qid> + use<'b, '_> {
        let dclause = d.to_clause();
        let clause_bss = self
            .clause_idxs
            .iter()
            .map(|idx| dclause.bs_from_idx(idx).into_iter())
            .collect_vec();

        ConjunctionIterator::new(clause_bss)
    }

    pub fn bs_from_document(&self, d: &Document) -> RoaringBitmap {
        // This is where the magic happens.
        let mut dclause = d.to_clause();
        // Add the match all to match all queries
        dclause.add_termquery(TermQuery::match_all());

        let mut clause_bss = self
            .clause_idxs
            .iter()
            .map(|idx| dclause.bs_from_idx(idx))
            .collect_vec();

        clause_bss.reverse();

        // There is at least one index, so at least one clause bitset
        let mut res = clause_bss.pop().unwrap();
        for other_bs in clause_bss.iter().rev() {
            res &= other_bs;
        }

        res
    }

    pub fn tracked_qids_from_document<'b>(
        &self,
        d: &'b Document,
    ) -> impl Iterator<Item = TrackedQid> + use<'b, '_> {
        // This is where the magic happens.
        let dclause = Clause::from_clauses(vec![d.to_clause(), Clause::match_all()]);

        // We are going to search this clause in all the clause indices
        let clause_its = self
            .clause_idxs
            .iter()
            .map(|idx| dclause.dids_from_idx(idx))
            .collect_vec();

        // And wrap all clauses into a ConjunctionIterator
        ConjunctionIterator::new(clause_its)
            // And a final filter, just to make sure.
            .enumerate()
            .filter(|&(_, query_id)| {
                // For each document ID, we check if it matches the query.
                // This is a bit inefficient, but we can optimize later.
                //println!("MULTIMATCH: {}", self.queries[query_id].to_cnf());
                self.queries[query_id as usize].matches(d)
            })
            .enumerate()
            .map(|(post_idx, (pre_idx, qid))| TrackedQid {
                pre_idx,
                post_idx,
                qid,
            })
    }
}

impl fmt::Display for MultiPercolator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MultiPerc-{}Qs/{}IDXs",
            self.queries.len(),
            self.clause_idxs.len()
        )
    }
}

impl MultiPercolator {
    ///
    /// Adds a query to this percolator. Will panic if
    /// there is more than u32::MAX queries.
    ///
    pub fn add_query(&mut self, q: Rc<dyn Query>) -> Qid {
        // Get the document from the query
        // and index in the query index.
        let cnf = q.to_cnf();

        let new_doc_id = self.queries.len();
        self.queries.push(q);

        // The Clause index is controlling the zip.
        self.clause_idxs
            .iter_mut()
            .zip(cnf_to_documents(&cnf))
            .for_each(|(idx, doc)| {
                //rintln!("For CNF={} -IDXDOC- {:?}", cnf, doc);
                idx.index_document(&doc);
                assert_eq!(idx.len(), self.queries.len());
            });

        self.cnf_queries.push(cnf);

        new_doc_id.try_into().unwrap()
    }

    pub fn get_query(&self, qid: Qid) -> Rc<dyn Query> {
        self.queries[qid as usize].clone()
    }

    pub fn qids_from_document<'a>(
        &self,
        d: &'a Document,
    ) -> impl Iterator<Item = Qid> + use<'_, 'a> {
        // This is where the magic happens.
        let mut dclause = d.to_clause();
        // Add the match all to match all queries
        dclause.add_termquery(TermQuery::match_all());

        // We are going to search this clause in all the clause indices
        // We indexed the matchall, so indices of a higher rank with no specific clause
        // for a given request will still match.
        let clause_its = self
            .clause_idxs
            .iter()
            .map(|idx| dclause.dids_from_idx(idx))
            .collect_vec();

        // And wrap all clauses into a ConjunctionIterator
        ConjunctionIterator::new(clause_its)
            // And a final filter, just to make sure.
            .filter(|&query_id| {
                // For each document ID, we check if it matches the query.
                // This is a bit inefficient, but we can optimize later.
                //println!("MULTIMATCH: {}", self.queries[query_id].to_cnf());
                self.cnf_queries[query_id as usize].matches(d)
            })
            .map(|x| x as Qid)
    }
}

mod test_cnf {

    #[test]
    fn test_empty() {
        use super::*;
        let cnf = CNFQuery::default();
        assert_eq!(cnf_to_documents(&cnf).next(), Some(Document::match_all()));
    }

    #[test]
    fn test_literal() {
        use super::*;
        let term_query = TermQuery::new("field".into(), "value".into());
        let cnf_query = CNFQuery::from_literal(term_query);
        let mut docs = cnf_to_documents(&cnf_query);
        assert_eq!(
            docs.next(),
            Some(Document::default().with_value("field", "value"))
        );
    }

    #[test]
    fn test_from_and() {
        use super::*;
        let term_query1 = TermQuery::new("field1".into(), "value1".into());
        let term_query2 = TermQuery::new("field2".into(), "value2".into());
        let cnf_query1 = CNFQuery::from_literal(term_query1);
        let cnf_query2 = CNFQuery::from_literal(term_query2);
        let combined = CNFQuery::from_and(vec![cnf_query1, cnf_query2]);
        // Structure would be:
        assert_eq!(
            combined.to_string(),
            "(AND (OR field1=value1) (OR field2=value2))"
        );
        let mut docs = cnf_to_documents(&combined);
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
        use super::*;
        let x = TermQuery::new("X".into(), "x".into());
        let y = TermQuery::new("Y".into(), "y".into());
        let cnf_query1 = CNFQuery::from_literal(x.clone());
        let cnf_query2 = CNFQuery::from_literal(y.clone());
        let combined = CNFQuery::from_or_two(cnf_query1, cnf_query2);

        let mut docs = cnf_to_documents(&combined);
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
        let z = TermQuery::new("Z".into(), "z".into());
        let q = CNFQuery::from_or_two(
            CNFQuery::from_and(vec![
                CNFQuery::from_literal(x.clone()),
                CNFQuery::from_literal(y.clone()),
            ]),
            CNFQuery::from_literal(z.clone()),
        );
        assert_eq!(q.to_string(), "(AND (OR X=x Z=z) (OR Y=y Z=z))");
        let mut docs = cnf_to_documents(&q);
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
