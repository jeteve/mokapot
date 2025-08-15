use std::collections::HashMap;
use std::rc::Rc;

#[allow(unused_imports)]
use itertools::Itertools;

use crate::models::queries::termdisjunction::TermDisjunction;
use crate::models::queries::TermQuery;

use crate::models::cnf::Clause;

#[derive(Debug, Default, Clone)]
pub struct Document {
    // Fields representing the document's content
    fields: HashMap<Rc<str>, Vec<Rc<str>>>,
    fvs_count: usize,
}

type DocFieldValue = (Rc<str>, Rc<str>);

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fv_count(&self) -> usize {
        self.fvs_count
    }

    pub fn to_clause(&self) -> Clause {
        Clause::from_termqueries(self.fv_pairs().map(|(f, v)| TermQuery::new(f, v)).collect())
    }

    pub fn to_percolator_query(&self) -> TermDisjunction {
        let mut doc_tqs: Vec<TermQuery> = Vec::with_capacity(self.fv_count());
        self.fv_pairs()
            .map(|(f, v)| TermQuery::new(f, v.clone()))
            .for_each(|tq| doc_tqs.push(tq));

        TermDisjunction::new(doc_tqs)
    }

    pub fn fv_pairs(&self) -> impl Iterator<Item = DocFieldValue> + use<'_> {
        self.fields()
            .map(|f| (f.clone(), self.field_values_iter(f.as_ref())))
            .filter_map(|(f, ovit)| ovit.map(|vit| (f, vit)))
            .flat_map(|(f, vit)| vit.map(move |v| (f.clone(), v)))
    }

    pub fn merge_with(&self, other: &Self) -> Self {
        // Find all the (key,value) of a document.
        self.fv_pairs()
            .chain(other.fv_pairs())
            .unique()
            .fold(Document::new(), |a, (f, v)| a.with_value(f, v.clone()))
    }

    pub fn with_value<T, U>(mut self, field: T, value: U) -> Self
    where
        T: Into<Rc<str>>,
        U: Into<Rc<str>> + Clone,
    {
        self.fields
            .entry(field.into())
            .and_modify(|v| v.push(value.clone().into()))
            .or_insert(vec![value.into()]);
        self.fvs_count += 1;
        self
    }

    pub fn fields(&self) -> impl Iterator<Item = Rc<str>> + use<'_> {
        self.fields.keys().cloned()
    }

    pub fn field_values(&self, field: &str) -> Vec<Rc<str>> {
        if let Some(it) = self.field_values_iter(field) {
            it.collect()
        } else {
            vec![]
        }
    }

    pub fn field_values_iter(&self, field: &str) -> Option<impl Iterator<Item = Rc<str>> + '_> {
        self.fields.get(field).map(|v| v.iter().cloned())
    }
}
