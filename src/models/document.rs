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

pub(crate) const MATCH_ALL: (&str, &str) = ("__match_all__", "true");

impl Document {
    /// Alias for default. An empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// A special document that only contain the match_all field,value
    pub fn match_all() -> Self {
        Self::default().with_value(MATCH_ALL.0, MATCH_ALL.1)
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
    pub fn merge_with(&self, other: &Self) -> Self {
        // Find all the (key,value) of a document.
        self.field_values()
            .chain(other.field_values())
            .unique()
            .fold(Document::new(), |a, (f, v)| a.with_value(f, v.clone()))
    }

    /// This document with a new field,value pair
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
