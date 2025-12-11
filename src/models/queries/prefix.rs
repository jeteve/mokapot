use crate::models::types::OurStr;
use crate::models::{document::Document, queries::common::DocMatcher};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct PrefixQuery {
    field: OurStr,
    prefix: OurStr,
}

impl PrefixQuery {
    /// Constructor
    pub(crate) fn new<T: Into<OurStr>, U: Into<OurStr>>(field: T, prefix: U) -> Self {
        PrefixQuery {
            field: field.into(),
            prefix: prefix.into(),
        }
    }

    /// The field
    pub(crate) fn field(&self) -> OurStr {
        self.field.clone()
    }

    /// The prefix
    pub(crate) fn prefix(&self) -> OurStr {
        self.prefix.clone()
    }
}

impl DocMatcher for PrefixQuery {
    /// Does this match the document?
    fn matches(&self, d: &Document) -> bool {
        d.values_iter(&self.field)
            .is_some_and(|mut i| i.any(|v| v.starts_with(self.prefix.as_ref())))
    }
}

mod test_prefix {
    use super::*;

    #[test]
    fn test_new_and_getters() {
        let field: OurStr = "test_field".into();
        let prefix: OurStr = "test_prefix".into();
        let q = PrefixQuery::new(field.clone(), prefix.clone());

        assert_eq!(q.field(), field);
        assert_eq!(q.prefix(), prefix);
    }

    #[test]
    fn test_matching() {
        let q = PrefixQuery::new("field", "pre");

        assert!(!q.matches(&Document::default()));
        assert!(!q.matches(&[("some", "thing")].into()));
        assert!(!q.matches(&[("field", "pr")].into()));
        assert!(q.matches(&[("field", "pre")].into()));
        assert!(q.matches(&[("field", "prescience")].into()));
        assert!(!q.matches(&[("field", "foo")].into()));
        assert!(!q.matches(&[("field", "")].into()));
    }
}
