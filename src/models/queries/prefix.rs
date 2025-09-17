use lean_string::LeanString;
use std::rc::Rc;

use crate::models::{document::Document, queries::common::DocMatcher};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct PrefixQuery {
    field: LeanString,
    prefix: LeanString,
}

impl PrefixQuery {
    /// Constructor
    pub(crate) fn new<T: Into<LeanString>, U: Into<LeanString>>(field: T, prefix: U) -> Self {
        PrefixQuery {
            field: field.into(),
            prefix: prefix.into(),
        }
    }

    /// The field
    pub(crate) fn field(&self) -> LeanString {
        self.field.clone()
    }

    /// The prefix
    pub(crate) fn prefix(&self) -> LeanString {
        self.prefix.clone()
    }
}

impl DocMatcher for PrefixQuery {
    /// Does this match the document?
    fn matches(&self, d: &Document) -> bool {
        d.values_iter(&self.field)
            .is_some_and(|mut i| i.any(|v| v.starts_with(self.prefix.as_str())))
    }
}

mod test_prefix {

    #[test]
    fn test_matching() {
        use super::*;
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
