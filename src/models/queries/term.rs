use roaring::RoaringBitmap;

use crate::models::document::Document;
use crate::models::document::MATCH_ALL;
use crate::models::index::*;

use std::rc::Rc;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TermQuery {
    field: Rc<str>,
    term: Rc<str>,
}

impl TermQuery {
    /// Constructor
    pub fn new<T: Into<Rc<str>>, U: Into<Rc<str>>>(field: T, term: U) -> Self {
        TermQuery {
            field: field.into(),
            term: term.into(),
        }
    }

    /// A match all term query. Just a special symbol.
    pub fn match_all() -> Self {
        TermQuery::new(MATCH_ALL.0, MATCH_ALL.1)
    }

    /// The field
    pub fn field(&self) -> Rc<str> {
        self.field.clone()
    }

    /// The term
    pub fn term(&self) -> Rc<str> {
        self.term.clone()
    }

    // Specialized method. Cannot be part of a trait for use of lifetime
    // in the concrete impl implementation.
    pub fn dids_from_idx<'a>(&self, index: &'a Index) -> impl Iterator<Item = DocId> + use<'a> {
        self.bs_from_idx(index).iter()
    }

    pub fn bs_from_idx<'a>(&self, index: &'a Index) -> &'a RoaringBitmap {
        index.term_bs(self.field.clone(), self.term.clone())
    }

    pub fn matches(&self, d: &Document) -> bool {
        d.iter_values(&self.field)
            .is_some_and(|mut i| i.any(|v| v == self.term))
    }
}
