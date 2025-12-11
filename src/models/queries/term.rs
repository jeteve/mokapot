use roaring::RoaringBitmap;

use crate::models::document::Document;
use crate::models::document::MATCH_ALL;
use crate::models::index::*;
use crate::models::queries::common::DocMatcher;

use crate::models::types::OurStr;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct TermQuery {
    field: OurStr,
    term: OurStr,
}

impl TermQuery {
    /// Constructor
    pub fn new<T: Into<OurStr>, U: Into<OurStr>>(field: T, term: U) -> Self {
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
    pub fn field(&self) -> OurStr {
        self.field.clone()
    }

    /// The term
    pub fn term(&self) -> OurStr {
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::document::MATCH_ALL;
    use crate::models::document::Document;

    #[test]
    fn test_new_and_getters() {
        let field: OurStr = "test_field".into();
        let term: OurStr = "test_term".into();
        let query = TermQuery::new(field.clone(), term.clone());

        assert_eq!(query.field(), field);
        assert_eq!(query.term(), term);
    }

    #[test]
    fn test_match_all() {
        let query = TermQuery::match_all();
        assert_eq!(query.field().as_ref(), MATCH_ALL.0);
        assert_eq!(query.term().as_ref(), MATCH_ALL.1);
    }

    #[test]
    fn test_docs_from_idx() {
        let mut index = Index::default();
        let doc = Document::default().with_value("field", "value");
        let doc_id = index.index_document(&doc);

        let query = TermQuery::new("field", "value");
        let bitmap = query.docs_from_idx(&index);

        assert!(bitmap.contains(doc_id));

        let query_miss = TermQuery::new("field", "other");
        let bitmap_miss = query_miss.docs_from_idx(&index);
        assert!(bitmap_miss.is_empty());
    }

    #[test]
    fn test_matches() {
        let query = TermQuery::new("field", "value");

        // Match case
        let doc_match = Document::default().with_value("field", "value");
        assert!(query.matches(&doc_match));

        // Match with multiple values
        let doc_match_multi = Document::default()
            .with_value("field", "other")
            .with_value("field", "value");
        assert!(query.matches(&doc_match_multi));

        // No match - different value
        let doc_mismatch = Document::default().with_value("field", "other");
        assert!(!query.matches(&doc_mismatch));

        // No match - missing field
        let doc_missing = Document::default().with_value("other_field", "value");
        assert!(!query.matches(&doc_missing));

        // No match - empty doc
        let doc_empty = Document::default();
        assert!(!query.matches(&doc_empty));
    }
}
