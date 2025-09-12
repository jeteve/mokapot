use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use itertools::Itertools;

use crate::models::cnf::Clause;
use crate::models::queries::TermQuery;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Document {
    // Fields representing the document's content
    fields: HashMap<Rc<str>, Vec<Rc<str>>>,
    fvs_count: usize,
}

type FieldValue = (Rc<str>, Rc<str>);

/// A trait for types that can be converted into a `Cow<Document>`.
/// This is used to provide a more ergonomic API for functions that
/// need a refernce to a `Document`.
pub trait ToCowDocument {
    /// Converts this type into a `Cow<Document>`.
    fn to_cow_document<'a>(&'a self) -> Cow<'a, Document>;
}

impl ToCowDocument for Document {
    fn to_cow_document<'a>(&'a self) -> Cow<'a, Document> {
        Cow::Borrowed(self)
    }
}

// For types that can be converted to Document (owns)
// Use a newtype or marker trait to avoid conflicts
pub trait IntoDocument: Into<Document> + Clone {}

impl<T> ToCowDocument for T
where
    T: IntoDocument,
{
    fn to_cow_document<'a>(&'a self) -> Cow<'a, Document> {
        Cow::Owned(self.clone().into())
    }
}

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
    pub fn match_all() -> Self {
        Self::default().with_value(MATCH_ALL.0, MATCH_ALL.1)
    }

    pub fn is_match_all(&self) -> bool {
        self.fvs_count == 1 && self.has_field(MATCH_ALL.0)
    }

    /// The number of (field,value) tuples in this document.
    pub fn fv_count(&self) -> usize {
        self.fvs_count
    }

    pub fn to_clause(&self) -> Clause {
        Clause::from_termqueries(
            self.field_values()
                .map(|(f, v)| TermQuery::new(f, v))
                .collect(),
        )
    }

    /// An iterator on all the (field,value) tuples of this document.
    pub fn field_values(&self) -> impl Iterator<Item = FieldValue> + use<'_> {
        self.fields()
            .map(|f| (f.clone(), self.values_iter(f.as_ref())))
            .filter_map(|(f, ovit)| ovit.map(|vit| (f, vit)))
            .flat_map(|(f, vit)| vit.map(move |v| (f.clone(), v)))
    }

    /// The merge of this document with another one.
    ///
    /// # Example:
    /// ```
    /// use mokapot::models::document::Document;
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
    /// use mokapot::models::document::Document;
    ///
    /// let d = Document::default().with_value("field", "value");
    /// assert_eq!(d.values("field"), vec!["value".into()]);
    /// ```
    ///
    pub fn with_value<T, U>(mut self, field: T, value: U) -> Self
    where
        T: Into<Rc<str>>,
        U: Into<Rc<str>> + Clone,
    {
        let val: Rc<str> = value.into();

        self.fields
            .entry(field.into())
            .and_modify(|v| v.push(val.clone()))
            .or_insert(vec![val]);
        self.fvs_count += 1;
        self
    }

    pub fn has_field(&self, f: &str) -> bool {
        self.fields.contains_key(f)
    }

    /// All fields of this document
    pub fn fields(&self) -> impl Iterator<Item = Rc<str>> + use<'_> {
        self.fields.keys().cloned()
    }

    /// All values of the field
    pub fn values(&self, field: &str) -> Vec<Rc<str>> {
        if let Some(it) = self.values_iter(field) {
            it.collect()
        } else {
            vec![]
        }
    }

    /// All values of the field if it exists
    pub fn values_iter(&self, field: &str) -> Option<impl Iterator<Item = Rc<str>> + '_> {
        self.fields.get(field).map(|v| v.iter().cloned())
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for Document
where
    K: Into<Rc<str>>,
    V: Into<Rc<str>> + Clone,
{
    fn from(arr: [(K, V); N]) -> Self {
        arr.into_iter()
            .fold(Document::default(), |a, (k, v)| a.with_value(k, v))
    }
}

impl<K, V, const N: usize> IntoDocument for [(K, V); N]
where
    K: Into<Rc<str>> + Clone,
    V: Into<Rc<str>> + Clone,
{
}
