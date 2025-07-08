use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct Document {
    // Fields representing the document's content
    fields: HashMap<Rc<str>, Vec<Rc<str>>>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_field(mut self, field: Rc<str>, value: Rc<str>) -> Self {
        self.fields
            .entry(field)
            .and_modify(|v| v.push(value.clone()))
            .or_insert(vec![value]);
        self
    }

    pub fn get_field_iter(&self, field: &str) -> Option<impl Iterator<Item = &Rc<str>>> {
        self.fields.get(field).map(|v| v.iter())
    }
}

pub trait Query: std::fmt::Debug {
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

#[derive(Debug, Clone)]
pub struct TermQuery {
    field: Rc<str>,
    term: Rc<str>,
}

impl TermQuery {
    pub fn new(field: Rc<str>, term: Rc<str>) -> Self {
        TermQuery { field, term }
    }
}

impl Query for TermQuery {
    fn matches(&self, d: &Document) -> bool {
        d.get_field_iter(&self.field)
            .map_or(false, |mut iter| iter.any(|value| value == &self.term))
    }
}
