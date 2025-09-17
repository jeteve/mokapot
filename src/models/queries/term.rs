use roaring::RoaringBitmap;

use crate::models::document::Document;
use crate::models::document::MATCH_ALL;
use crate::models::index::*;
use crate::models::queries::common::DocMatcher;

use lean_string::LeanString;
use std::rc::Rc;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct TermQuery {
    field: LeanString,
    term: LeanString,
}

impl TermQuery {
    /// Constructor
    pub fn new<T: Into<LeanString>, U: Into<LeanString>>(field: T, term: U) -> Self {
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
    pub fn field(&self) -> LeanString {
        self.field.clone()
    }

    /// The term
    pub fn term(&self) -> LeanString {
        self.term.clone()
    }

    /// Bitmap of matching documents from the given index.
    pub(crate) fn docs_from_idx<'a>(&self, index: &'a Index) -> &'a RoaringBitmap {
        index.docs_from_fv(self.field.clone(), self.term.clone())
    }
}

impl DocMatcher for TermQuery {
    /// Does this match the document?
    fn matches(&self, d: &Document) -> bool {
        d.values_iter(&self.field)
            .is_some_and(|mut i| i.any(|v| v == self.term))
    }
}
