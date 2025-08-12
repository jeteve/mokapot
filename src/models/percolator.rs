use std::fmt;
use std::rc::Rc;

use itertools::Itertools;

use crate::models::queries::DisjunctionQuery;
use crate::models::queries::TermQuery;

use crate::models::{documents::Document, index::Index, queries::Query};

pub type Qid = usize;

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
        let doc_id = self
            .qindex
            //.index_document(&q.to_document());
            .index_document(&q.to_document_with_sample(&self.sample_docs));
        self.queries.push(q);

        assert_eq!(self.queries.len(), self.qindex.len());

        doc_id
    }

    ///
    /// Uses the specially optimised TermDisjunction that doesn't use dynamic objects.
    /// as a Document ALWAYS turn into a TermDisjunction anyway.
    pub fn static_qids_from_document<'b>(
        &self,
        d: &'b Document,
    ) -> impl Iterator<Item = Qid> + use<'b, '_> {
        d.to_percolator_query()
            .dids_from_idx(&self.qindex)
            .filter(|v| self.queries[*v].matches(d))
    }

    pub fn qids_from_document<'b>(
        &self,
        d: &'b Document,
    ) -> impl Iterator<Item = Qid> + use<'b, '_> {
        // First turn all unique field values into a disjunction of
        // term queries.

        let doc_tqs = d
            .fv_pairs()
            // Force to be a Box<dyn Query>
            // As type inference does go through the iterator item.
            .map(|(f, v)| Box::new(TermQuery::new(f, v.clone())) as Box<dyn Query>)
            .collect_vec();
        let doc_disj = DisjunctionQuery::new(doc_tqs);

        doc_disj
            .docids_from_index(&self.qindex)
            .filter(|v| self.queries[*v].matches(d))
    }
}
