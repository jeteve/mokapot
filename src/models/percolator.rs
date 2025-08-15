use std::fmt;
use std::rc::Rc;

use itertools::Itertools;

use crate::models::{documents::Document, index::Index, queries::Query};

pub type Qid = usize;

#[derive(Debug)]
pub struct MultiPercolator {
    queries: Vec<Rc<dyn Query>>,
    clause_idxs: Vec<Index>,
}

impl std::default::Default for MultiPercolator {
    fn default() -> Self {
        Self {
            queries: Vec::new(),
            clause_idxs: (0..5).map(|_| Index::new()).collect(),
        }
    }
}

impl fmt::Display for MultiPercolator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Percolator-{} queries over {} clause_idxs",
            self.queries.len(),
            self.clause_idxs.len()
        )
    }
}

impl MultiPercolator {
    pub fn add_query(&mut self, q: Rc<dyn Query>) -> Qid {
        // Get the document from the query
        // and index in the query index.
        let cnf = q.to_cnf();

        let new_doc_id = self.queries.len();
        self.queries.push(q);

        // Control numbers in clause indexes
        self.clause_idxs
            .iter_mut()
            .zip(cnf.to_documents())
            .for_each(|(idx, doc)| {
                idx.index_document(&doc);
                assert_eq!(idx.len(), self.queries.len());
            });

        new_doc_id
    }
}

#[derive(Default, Debug)]
pub struct Percolator {
    qindex: Index,
    sample_docs: Index,
    // The box of query objects.
    queries: Vec<Rc<dyn Query>>,
}

impl fmt::Display for Percolator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Percolator-{} queries", self.queries.len())
    }
}

impl Percolator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_sample_document(&mut self, d: &Document) {
        self.sample_docs.index_document(d);
    }

    pub fn get_query(&self, qid: Qid) -> Rc<dyn Query> {
        self.queries[qid].clone()
    }

    pub fn add_query(&mut self, q: Rc<dyn Query>) -> Qid {
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
    pub fn qids_from_document<'b>(
        &self,
        d: &'b Document,
    ) -> impl Iterator<Item = Qid> + use<'b, '_> {
        d.to_clause()
            .dids_from_idx(&self.qindex)
            .filter(|v| self.queries[*v].matches(d))
            .sorted()
    }
}
