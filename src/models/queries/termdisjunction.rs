use crate::models::documents::Document;
use crate::models::index::{DocId, Index};
use crate::models::iterators::DisjunctionIterator;
use crate::models::queries::query::Query;
use crate::models::queries::TermQuery;

// A Specialised Term disjunction query
// for use by the Percolator.
#[derive(Debug)]
pub struct TermDisjunction {
    queries: Vec<TermQuery>,
}
impl TermDisjunction {
    pub fn new(queries: Vec<TermQuery>) -> Self {
        TermDisjunction { queries }
    }

    pub fn matches(&self, d: &Document) -> bool {
        self.queries.iter().any(|q| q.matches(d))
    }

    pub fn dids_from_idx<'a>(&self, index: &'a Index) -> impl Iterator<Item = DocId> + use<'a> {
        let iterators: Vec<_> = self
            .queries
            //.sort_by_cached_key(|q);
            .iter()
            .map(|q| q.dids_from_idx(index))
            .collect();
        DisjunctionIterator::new(iterators)
    }
}
