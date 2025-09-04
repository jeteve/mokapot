use super::query::*;
use crate::models::cnf::CNFQuery;
use crate::models::documents::Document;
use crate::models::index::*;
use crate::models::iterators;

#[derive(Debug)]
pub struct ConjunctionQuery(CNFQuery);
impl ConjunctionQuery {
    pub fn new(queries: Vec<Box<dyn Query>>) -> Self {
        ConjunctionQuery(CNFQuery::from_and(
            queries.into_iter().map(|q| q.to_cnf()).collect(),
        ))
    }
}

impl Query for ConjunctionQuery {
    fn matches(&self, d: &Document) -> bool {
        self.0.matches(d)
    }

    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        // Get iterators from all queries
        let iterators: Vec<_> = self
            .0
            .clauses()
            .iter()
            .map(|q| q.dids_from_idx(index))
            .collect();

        Box::new(iterators::ConjunctionIterator::new(iterators))
    }

    /**
    Turns this into a CNF query for indexing into a multi percolator.
     */
    fn to_cnf(&self) -> CNFQuery {
        self.0.clone()
    }
}
