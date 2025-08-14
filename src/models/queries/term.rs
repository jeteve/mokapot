use crate::models::documents::Document;
use crate::models::index::*;
use crate::models::queries::query::*;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct TermQuery {
    field: Rc<str>,
    term: Rc<str>,
}

impl TermQuery {
    pub fn new(field: Rc<str>, term: Rc<str>) -> Self {
        TermQuery { field, term }
    }

    pub fn field(&self) -> Rc<str> {
        self.field.clone()
    }

    pub fn term(&self) -> Rc<str> {
        self.term.clone()
    }

    // Specialized method. Cannot be part of a trait for use of lifetime
    // in the concrete impl implementation.
    pub fn dids_from_idx<'a>(&self, index: &'a Index) -> impl Iterator<Item = DocId> + use<'a> {
        index.term_iter(self.field.clone(), self.term.clone())
    }
}

impl Query for TermQuery {
    fn doc_enrichers(&self) -> Vec<DocPredicate> {
        // vec![DocPredicate {
        //     name: "{self.field}_is_{self.term}".into(),
        //     query: self,
        // }]
        vec![]
    }

    fn matches(&self, d: &Document) -> bool {
        d.field_values_iter(&self.field)
            .is_some_and(|mut i| i.any(|v| v == self.term))
    }

    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        Box::new(self.dids_from_idx(index))
    }

    fn to_document(&self) -> Document {
        Document::default().with_value(self.field.clone(), self.term.clone())
    }

    fn specificity(&self) -> f64 {
        1.0
    }
}
