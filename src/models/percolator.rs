use std::rc::Rc;

use itertools::Itertools;

use crate::models::queries::DisjunctionQuery;
use crate::models::queries::TermQuery;

use crate::models::{documents::Document, index::Index, queries::Query};

pub type Qid = usize;

#[derive(Default)]
pub struct Percolator {
    qindex: Index,
    // The box of query objects.
    queries: Vec<Rc<dyn Query>>,
}

impl Percolator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_query(&mut self, q: Rc<dyn Query>) -> Qid {
        // Get the document from the query
        // and index in the query index.
        let doc_id = self.qindex.index_document(q.to_document());
        self.queries.push(q);

        assert_eq!(self.queries.len(), self.qindex.len());

        doc_id
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
