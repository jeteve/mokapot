use super::query::*;
use crate::models::cnf::CNFQuery;
use crate::models::documents::Document;
use crate::models::index::{DocId, Index};
use itertools::*;

#[derive(Debug)]
pub struct DisjunctionQuery(CNFQuery);

impl DisjunctionQuery {
    pub fn new(queries: Vec<Box<dyn Query>>) -> Self {
        DisjunctionQuery(CNFQuery::from_or(
            queries.into_iter().map(|q| q.to_cnf()).collect(),
        ))
    }
}

impl Query for DisjunctionQuery {
    fn matches(&self, d: &Document) -> bool {
        self.0.matches(d)
    }

    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        let iterators: Vec<_> = self
            .0
            .clauses()
            .iter()
            .map(|q| q.dids_from_idx(index))
            .collect();
        Box::new(itertools::kmerge(iterators).dedup())
        //Box::new(iterators::DisjunctionIterator::new(iterators))
    }

    /* fn to_document(&self) -> Document {
        // Returns all fields from the queries,
        // as the incoming document will turn into a Disjunction query.
        self.0
            .clauses()
            .iter()
            .fold(Document::default(), |a, q| a.merge_with(&q.to_document()))
    } */

    fn to_cnf(&self) -> CNFQuery {
        self.0.clone()
    }
}
