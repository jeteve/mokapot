use crate::models::index;

use super::documents::Document;
use std::rc::Rc;

pub trait Query: std::fmt::Debug {
    //fn filter_index<'a>(&self, index: &'a index::Index) -> impl Iterator<Item = index::DocId> + 'a;
    fn matches(&self, d: &Document) -> bool;
}

#[derive(Debug)]
pub struct ConjunctionQuery {
    queries: Vec<Box<dyn Query>>,
}
impl ConjunctionQuery {
    pub fn new(queries: Vec<Box<dyn Query>>) -> Self {
        ConjunctionQuery { queries }
    }
}

impl Query for ConjunctionQuery {
    fn matches(&self, d: &Document) -> bool {
        self.queries.iter().all(|q| q.matches(d))
    }
}

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
}

#[derive(Debug, Clone)]
pub struct TermQuery {
    field: Rc<str>,
    term: Rc<str>,
}

impl TermQuery {
    pub fn new(field: Rc<str>, term: Rc<str>) -> Self {
        TermQuery { field, term }
    }
    pub fn filter_index<'a>(
        &self,
        index: &'a index::Index,
    ) -> impl Iterator<Item = index::DocId> + 'a {
        index.term_iter(self.field.clone(), self.term.clone())
    }
}

impl Query for TermQuery {
    fn matches(&self, d: &Document) -> bool {
        d.field_values_iter(&self.field)
            .map_or(false, |mut iter| iter.any(|value| value == &self.term))
    }
}
