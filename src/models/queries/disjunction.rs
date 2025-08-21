use super::query::*;
use crate::models::cnf::CNFQuery;
use crate::models::documents::Document;
use crate::models::index::{DocId, Index};
use itertools::*;

#[derive(Debug)]
pub struct DisjunctionQuery {
    queries: Vec<Box<dyn Query>>,
}
impl DisjunctionQuery {
    pub fn new(queries: Vec<Box<dyn Query>>) -> Self {
        DisjunctionQuery { queries }
    }
}
impl Query for DisjunctionQuery {
    fn matches(&self, d: &Document) -> bool {
        self.queries.iter().any(|q| q.matches(d))
    }

    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        let iterators: Vec<_> = self
            .queries
            .iter()
            .map(|q| q.docids_from_index(index))
            .collect();
        Box::new(itertools::kmerge(iterators).dedup())
        //Box::new(iterators::DisjunctionIterator::new(iterators))
    }

    fn to_document(&self) -> Document {
        // Returns all fields from the queries,
        // as the incoming document will turn into a Disjunction query.
        self.queries
            .iter()
            .fold(Document::default(), |a, q| a.merge_with(&q.to_document()))
    }

    fn specificity(&self) -> f64 {
        // Guarantee No NAN.
        if self.queries.is_empty() {
            0.0
        } else {
            // Just the Max of the sub specificities (or zero if nothing)
            // divided by the length.
            // This is because a disjunction is less specific that its individual queries.
            self.queries
                .iter()
                .map(|q| q.specificity())
                .fold(f64::NAN, f64::max)
                / self.queries.len() as f64
        }
    }

    fn to_cnf(&self) -> CNFQuery {
        CNFQuery::from_or(self.queries.iter().map(|q| q.to_cnf()).collect())
    }
}
