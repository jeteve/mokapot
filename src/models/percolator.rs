use std::fmt;
use std::rc::Rc;

use itertools::Itertools;

use crate::models::{
    cnf::Clause,
    documents::Document,
    index::Index,
    iterators::ConjunctionIterator,
    queries::{Query, TermQuery},
};

pub type Qid = usize;

pub trait Percolator: fmt::Display {
    /**
    Index a query in the percolator
    */
    fn add_query(&mut self, q: Rc<dyn Query>) -> Qid;

    /**
    Get the query by the ID returned earlier.
    Panics if the ID is invalid.
    */
    fn get_query(&self, qid: Qid) -> Rc<dyn Query>;

    /**
    The Query IDs matching the given Document as an iterator.

    This is the *main* feature of a percolator.
    */
    fn qids_from_document(&self, d: &Document) -> impl Iterator<Item = Qid>;

    /*
    The specificity of the indexing strategy. Its more an internal metric really.
     */
    //fn indexing_specificity(&self) -> f64;
}

#[derive(Debug)]
pub struct MultiPercolator {
    queries: Vec<Rc<dyn Query>>,
    clause_idxs: Vec<Index>,
}

impl std::default::Default for MultiPercolator {
    fn default() -> Self {
        Self {
            queries: Vec::new(),
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
                self.queries[query_id].matches(d)
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

impl Percolator for MultiPercolator {
    fn add_query(&mut self, q: Rc<dyn Query>) -> Qid {
        // Get the document from the query
        // and index in the query index.
        let cnf = q.to_cnf();

        let new_doc_id = self.queries.len();
        self.queries.push(q);

        // The Clause index is controlling the zip.
        self.clause_idxs
            .iter_mut()
            .zip(cnf.to_documents())
            .for_each(|(idx, doc)| {
                //rintln!("For CNF={} -IDXDOC- {:?}", cnf, doc);
                idx.index_document(&doc);
                assert_eq!(idx.len(), self.queries.len());
            });

        new_doc_id
    }

    fn get_query(&self, qid: Qid) -> Rc<dyn Query> {
        self.queries[qid].clone()
    }

    fn qids_from_document(&self, d: &Document) -> impl Iterator<Item = Qid> {
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
                self.queries[query_id].matches(d)
            })
    }
}

#[derive(Default, Debug)]
pub struct SimplePercolator {
    qindex: Index,
    sample_docs: Index,
    // The box of query objects.
    queries: Vec<Rc<dyn Query>>,
}

impl fmt::Display for SimplePercolator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SimplePerc-{}Qs", self.queries.len())
    }
}

impl SimplePercolator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_sample_document(&mut self, d: &Document) {
        self.sample_docs.index_document(d);
    }

    pub fn tracked_qids_from_document<'b>(
        &self,
        d: &'b Document,
    ) -> impl Iterator<Item = TrackedQid> + use<'b, '_> {
        d.to_clause()
            .dids_from_idx(&self.qindex)
            .enumerate()
            .filter(|(_, qid)| {
                //println!("SIMPLEMATCH: {}", self.queries[*qid].to_cnf());
                self.queries[*qid].matches(d)
            })
            .enumerate()
            .map(|(post_idx, (pre_idx, qid))| TrackedQid {
                pre_idx,
                post_idx,
                qid,
            })
    }
}

impl Percolator for SimplePercolator {
    fn get_query(&self, qid: Qid) -> Rc<dyn Query> {
        self.queries[qid].clone()
    }

    fn add_query(&mut self, q: Rc<dyn Query>) -> Qid {
        // Get the document from the query
        // and index in the query index.
        let doc_id = self.qindex.index_document(&q.to_document());
        //.index_document(&q.to_document_with_sample(&self.sample_docs));
        self.queries.push(q);

        assert_eq!(self.queries.len(), self.qindex.len());

        doc_id
    }

    ///
    /// Uses the specially optimised TermDisjunction that doesn't use dynamic objects.
    /// as a Document ALWAYS turn into a TermDisjunction anyway.
    fn qids_from_document(&self, d: &Document) -> impl Iterator<Item = Qid> {
        self.tracked_qids_from_document(d).map(|tq| tq.qid)
    }
}
