use std::{rc::Rc, str::FromStr};

use crate::models::queries::common::DocMatcher;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Ordering {
    GT,
    LT,
    GE,
    LE,
    EQ,
}
impl Ordering {
    fn compare<T: PartialOrd>(&self, a: &T, b: &T) -> bool {
        match &self {
            Ordering::GT => a > b,
            Ordering::LT => a < b,
            Ordering::GE => a >= b,
            Ordering::LE => a <= b,
            Ordering::EQ => a == b,
        }
    }
}

///
/// Represents a query about partially ordered elements.
///
/// The fields it compares to are strings as usual and are parsed as the
/// comparison type on exact comparisons.
///
/// Over/Underflow values will NOT match.
///
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct OrderedQuery<T: PartialOrd + FromStr> {
    field: Rc<str>,
    cmp_point: T,
    cmp_ord: Ordering,
}

/// Aliases for convenience.
pub(crate) type I32Query = OrderedQuery<i32>;
pub(crate) type I64Query = OrderedQuery<i64>;

pub(crate) type U32Query = OrderedQuery<u32>;
pub(crate) type U64Query = OrderedQuery<u64>;

impl<T: PartialOrd + FromStr> OrderedQuery<T> {
    pub(crate) fn new<F: Into<Rc<str>>>(field: F, cmp_point: T, cmp_ord: Ordering) -> Self {
        OrderedQuery {
            field: field.into(),
            cmp_point,
            cmp_ord,
        }
    }
}

impl<T: PartialOrd + FromStr> DocMatcher for OrderedQuery<T> {
    fn matches(&self, d: &crate::prelude::Document) -> bool {
        d.values_iter(&self.field).is_some_and(|mut i| {
            i.any(|v| {
                v.parse()
                    .is_ok_and(|iv: T| self.cmp_ord.compare(&iv, &self.cmp_point))
            })
        })
    }
}

#[cfg(test)]
mod test_prefix {
    use super::*;
    use crate::prelude::Document;

    #[test]
    #[cfg(feature = "serde")]
    fn test_serialize() {
        let q = I64Query::new("field", 123, Ordering::EQ);
        let json = serde_json::to_string(&q).unwrap();
        let q2 = serde_json::from_str(&json).unwrap();
        assert_eq!(q, q2);
    }

    #[test]
    fn test_equality() {
        let q = I64Query::new("field", 123, Ordering::EQ);

        assert!(!q.matches(&Document::default()));
        assert!(!q.matches(&[("some", "thing")].into()));
        assert!(!q.matches(&[("field", "not number")].into()));
        assert!(q.matches(&[("field", "123")].into()));
        assert!(q.matches(&[("field", "0000000123"), ("field", "not number")].into()));
        assert!(!q.matches(&[("field", "foo")].into()));
        assert!(!q.matches(&[("field", "")].into()));
    }

    #[test]
    fn test_lt() {
        let q = I64Query::new("field", 123, Ordering::LT);

        assert!(!q.matches(&Document::default()));
        assert!(q.matches(&[("field", "122")].into()));
        assert!(!q.matches(&[("field", "123")].into()));
        assert!(!q.matches(&[("field", "124")].into()));

        assert!(q.matches(&[("field", "0000000122"), ("field", "not number")].into()));
        assert!(!q.matches(&[("field", "foo")].into()));
        assert!(!q.matches(&[("field", "")].into()));

        let q = OrderedQuery::<f64>::new("field", 1.23, Ordering::LT);
        assert!(!q.matches(&Document::default()));
        assert!(q.matches(&[("field", "1.22000")].into()));
        assert!(!q.matches(&[("field", "1.23")].into()));
        assert!(!q.matches(&[("field", "1.24")].into()));

        assert!(q.matches(&[("field", "00000001.22000000000"), ("field", "not number")].into()));
        assert!(!q.matches(&[("field", "foo")].into()));
        assert!(!q.matches(&[("field", "")].into()));
    }

    #[test]
    fn test_le() {
        let q = I64Query::new("field", 123, Ordering::LE);

        assert!(!q.matches(&Document::default()));
        assert!(q.matches(&[("field", "122")].into()));
        assert!(q.matches(&[("field", "123")].into()));
        assert!(!q.matches(&[("field", "124")].into()));

        assert!(q.matches(&[("field", "0000000122"), ("field", "not number")].into()));
        assert!(!q.matches(&[("field", "foo")].into()));
        assert!(!q.matches(&[("field", "")].into()));
    }

    #[test]
    fn test_gt() {
        let q = I64Query::new("field", 123, Ordering::GT);

        assert!(!q.matches(&Document::default()));
        assert!(!q.matches(&[("field", "122")].into()));
        assert!(!q.matches(&[("field", "123")].into()));
        assert!(q.matches(&[("field", "124")].into()));

        assert!(!q.matches(&[("field", "not number"), ("field", "0000000122")].into()));
        assert!(q.matches(&[("field", "not number"), ("field", "0000000124")].into()));
        assert!(!q.matches(&[("field", "foo")].into()));
        assert!(!q.matches(&[("field", "")].into()));
    }

    #[test]
    fn test_ge() {
        let q = OrderedQuery::<u64>::new("field", 123, Ordering::GE);

        assert!(!q.matches(&Document::default()));
        assert!(!q.matches(&[("field", "122")].into()));
        assert!(q.matches(&[("field", "123")].into()));
        assert!(q.matches(&[("field", "124")].into()));

        assert!(!q.matches(&[("field", "not number"), ("field", "0000000122")].into()));
        assert!(q.matches(&[("field", "not number"), ("field", "0000000123")].into()));
        assert!(q.matches(&[("field", "not number"), ("field", "0000000124")].into()));
        assert!(!q.matches(&[("field", "foo")].into()));
        assert!(!q.matches(&[("field", "")].into()));

        let q = OrderedQuery::<String>::new("field", "banana".into(), Ordering::GE);
        assert!(!q.matches(&[("field", "abc")].into()));
        assert!(q.matches(&[("field", "banana")].into()));
        assert!(q.matches(&[("field", "cart")].into()));
    }

    #[test]
    fn test_overflow() {
        let q = OrderedQuery::<u8>::new("field", 123, Ordering::GE);

        assert!(!q.matches(&Document::default()));
        assert!(!q.matches(&[("field", "122")].into()));
        assert!(q.matches(&[("field", "123")].into()));
        assert!(q.matches(&[("field", "124")].into()));
        assert!(q.matches(&[("field", "255")].into()));
        // an overflow value does not match
        assert!(!q.matches(&[("field", "256")].into()));

        assert!(!q.matches(&[("field", "not number"), ("field", "0000000122")].into()));
        assert!(q.matches(&[("field", "not number"), ("field", "0000000123")].into()));
        assert!(q.matches(&[("field", "not number"), ("field", "0000000124")].into()));
        assert!(!q.matches(&[("field", "foo")].into()));
        assert!(!q.matches(&[("field", "")].into()));
    }
}
