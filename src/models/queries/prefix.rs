use std::rc::Rc;

use crate::models::document::Document;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PrefixQuery {
    field: Rc<str>,
    prefix: Rc<str>,
}

impl PrefixQuery {
    /// Constructor
    pub fn new<T: Into<Rc<str>>, U: Into<Rc<str>>>(field: T, prefix: U) -> Self {
        PrefixQuery {
            field: field.into(),
            prefix: prefix.into(),
        }
    }

    /// The field
    pub fn field(&self) -> Rc<str> {
        self.field.clone()
    }

    /// The prefix
    pub fn prefix(&self) -> Rc<str> {
        self.prefix.clone()
    }

    /// Does this match the document?
    pub fn matches(&self, d: &Document) -> bool {
        d.values_iter(&self.field)
            .is_some_and(|mut i| i.any(|v| v.starts_with(self.prefix.as_ref())))
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
