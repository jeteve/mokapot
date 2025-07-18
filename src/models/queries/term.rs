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
}

impl Query for TermQuery {
    fn doc_enrichers(&self) -> Vec<DocPredicate> {
        vec![DocPredicate {
            name: "{self.field}_is_{self.term}".into(),
            query: self,
        }]
    }

    fn matches(&self, d: &Document) -> bool {
        d.field_values_iter(&self.field)
            .map_or(false, |mut iter| iter.any(|value| value == self.term))
    }
    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        Box::new(index.term_iter(self.field.clone(), self.term.clone()))
    }

    fn to_document(&self) -> Document {
        Document::default().with_value(self.field.clone(), self.term.clone())
    }

    fn specificity(&self) -> f64 {
        1.0
    }
}
