use hashbrown::HashMap;

use itertools::Itertools;

use crate::models::cnf::Clause;
use crate::models::queries::term::TermQuery;
use crate::models::types::OurStr;

/// A Document is what you build to percolate through the set of queries
/// using a Percolator. A document is simply a multimap of (field,value)
///
/// You can build an example dynamically as follow:
///
/// ```
/// use mokaccino::prelude::*;
///
/// let d = Document::default().with_value("field", "value")
///                             .with_value("field", "second_value")
///                             .with_value("field2", "another_value");
///
/// ```
///
/// Or to make it easier for simple cases, from an array of tuples:
///
/// ```
/// use mokaccino::prelude::*;
///
/// let d: Document = [("field", "value"), ("field", "another_value")].into();
/// ```
///
///
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Document {
    // Fields representing the document's content
    fields: HashMap<OurStr, Vec<OurStr>>,
    fvs_count: usize,
}

type FieldValue = (OurStr, OurStr);

pub(crate) const MATCH_ALL: (&str, &str) = ("__match_all__", "true");

impl Document {
    /// Alias for default. An empty document.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.fvs_count == 0
    }

    /// A special document that only contain the match_all field,value
    pub(crate) fn match_all() -> Self {
        Self::default().with_value(MATCH_ALL.0, MATCH_ALL.1)
    }

    pub(crate) fn is_match_all(&self) -> bool {
        self.fvs_count == 1 && self.has_field(MATCH_ALL.0)
    }

    /// The number of (field,value) tuples in this document.
    pub fn fv_count(&self) -> usize {
        self.fvs_count
    }

    pub(crate) fn to_clause(&self) -> Clause {
        Clause::from_termqueries(
            self.field_values()
                .map(|(f, v)| TermQuery::new(f, v))
                .collect(),
        )
    }

    /// An iterator on all the (field,value) tuples of this document.
    /// In no particular order.
    pub fn field_values(&self) -> impl Iterator<Item = FieldValue> + use<'_> {
        self.fields.iter().flat_map(|(field, values)| {
            values
                .iter()
                .map(move |value| (field.clone(), value.clone()))
        })
    }

    /// The merge of this document with another one.
    ///
    /// # Example:
    /// ```
    /// use mokaccino::models::document::Document;
    ///
    /// let d1 = Document::default().with_value("A", "a");
    /// let d2 = Document::default().with_value("A", "a2").with_value("B", "b");
    ///
    /// let d3 = d1.merge_with(&d2);
    /// assert_eq!(d3.values("A"), vec!["a".into(), "a2".into()]);
    /// assert_eq!(d3.values("B"), vec!["b".into()]);
    /// ```
    pub fn merge_with(&self, other: &Self) -> Self {
        // Find all the (key,value) of a document.
        self.field_values()
            .chain(other.field_values())
            .unique()
            .fold(Document::new(), |a, (f, v)| a.with_value(f, v.clone()))
    }

    /// This document with a new field,value
    ///
    /// # Example:
    /// ```
    /// use mokaccino::models::document::Document;
    ///
    /// let d = Document::default().with_value("field", "value");
    /// assert_eq!(d.values("field"), vec!["value".into()]);
    /// ```
    ///
    pub fn with_value<T, U>(mut self, field: T, value: U) -> Self
    where
        T: Into<OurStr>,
        U: Into<OurStr>,
    {
        self.with_value_mut(field, value);
        self
    }

    /// This document with a new field,value, by mutable reference.
    /// This is more memory-efficient than `with_value` when called in a loop
    /// as it avoids moving the `Document` instance.
    /// # Example:
    /// ```
    /// use mokaccino::models::document::Document;
    /// let mut d = Document::default();
    /// d.with_value_mut("field", "value");
    /// assert_eq!(d.values("field"), vec!["value".into()]);
    /// ```
    ///
    pub fn with_value_mut<T, U>(&mut self, field: T, value: U)
    where
        T: Into<OurStr>,
        U: Into<OurStr>,
    {
        let val: OurStr = value.into();

        self.fields.entry(field.into()).or_default().push(val);
        self.fvs_count += 1;
    }

    pub fn has_field(&self, f: &str) -> bool {
        self.fields.contains_key(f)
    }

    /// All fields of this document
    /// in no particular order.
    pub fn fields(&self) -> impl Iterator<Item = OurStr> + use<'_> {
        self.fields.keys().cloned()
    }

    /// The values of the field, if present in the document.
    pub fn values_ref(&self, field: &str) -> Option<&Vec<OurStr>> {
        self.fields.get(field)
    }

    /// All values of the field
    pub fn values(&self, field: &str) -> Vec<OurStr> {
        self.fields.get(field).cloned().unwrap_or_default()
    }

    /// All values of the field if it exists
    pub fn values_iter(&self, field: &str) -> Option<impl Iterator<Item = OurStr> + '_ + use<'_>> {
        self.fields.get(field).map(|v| v.iter().cloned())
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for Document
where
    K: Into<OurStr>,
    V: Into<OurStr> + Clone,
{
    fn from(arr: [(K, V); N]) -> Self {
        arr.into_iter()
            .fold(Default::default(), |a, (k, v)| a.with_value(k, v))
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::Document;

    #[test]
    fn test_from() {
        use super::*;
        use std::convert::Into;

        assert_eq!(
            Document::from([("taste", "sweet"), ("colour", "blue")]),
            Document::default()
                .with_value("taste", "sweet")
                .with_value("colour", "blue")
        );

        assert_eq!(
            Into::<Document>::into([("colour", "blue")]),
            Document::default().with_value("colour", "blue")
        );
    }

    #[test]
    fn test_document_merge() {
        use super::*;
        let d1 = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "bitter");
        let d2 = Document::default()
            .with_value("colour", "beige")
            .with_value("colour", "blue");

        let d3 = d1.merge_with(&d2);

        assert_eq!(d3.values("size").len(), 0);
        assert_eq!(d3.values("colour"), vec!["blue".into(), "beige".into()]);
        assert_eq!(d3.values("taste"), vec!["bitter".into()]);
    }

    #[test]
    fn test_document_to_clause() {
        use super::*;
        let d = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "bitter")
            .with_value("taste", "sweet");
        assert!(!d.is_empty());

        let clause = d.to_clause();
        assert_eq!(
            clause.to_string(),
            "(OR colour=blue taste=bitter taste=sweet)"
        );

        let d = Document::default();
        assert_eq!(d.to_clause().to_string(), "(OR )");
    }

    #[test]
    fn test_basics() {
        let d = Document::default();
        assert!(d.is_empty());
        assert_eq!(d.fv_count(), 0);
        assert!(d.values_ref("field").is_none());
    }
}
