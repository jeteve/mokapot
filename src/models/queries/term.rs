use roaring::RoaringBitmap;

use crate::models::document::Document;
use crate::models::document::MATCH_ALL;
use crate::models::index::*;
use crate::models::queries::Query;

use std::rc::Rc;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct TermQuery {
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

    /// Bitmap of matching documents from the given index.
    pub(crate) fn docs_from_idx<'a>(&self, index: &'a Index) -> &'a RoaringBitmap {
        index.docs_from_fv(self.field.clone(), self.term.clone())
    }
}

impl Query for TermQuery {
    /// Does this match the document?
    fn matches(&self, d: &Document) -> bool {
        d.values_iter(&self.field)
            .is_some_and(|mut i| i.any(|v| v == self.term))
    }
}
