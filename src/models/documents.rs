use std::collections::HashMap;
use std::rc::Rc;

#[allow(unused_imports)]
use itertools::Itertools;

#[derive(Debug, Default, Clone)]
pub struct Document {
    // Fields representing the document's content
    fields: HashMap<Rc<str>, Vec<Rc<str>>>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge_with(&self, other: &Self) -> Self {
        // Find all the (key,value) of a document.
        fn fvs(d: &Document) -> impl Iterator<Item = (Rc<str>, &Rc<str>)> {
            d.fields()
                .map(|f| (f, d.field_values_iter(f)))
                .filter_map(|(f, ovit)| ovit.map(|vit| (f, vit)))
                .flat_map(|(f, vit)| vit.map(|v| (f.clone(), v)))
        }

        fvs(self)
            .chain(fvs(other))
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
        self
    }

    pub fn fields(&self) -> impl Iterator<Item = &Rc<str>> {
        self.fields.keys()
    }

    pub fn field_values(&self, field: &str) -> Vec<&str> {
        if let Some(it) = self.field_values_iter(field) {
            it.map(|v| v.as_ref()).collect()
        } else {
            vec![]
        }
    }

    pub fn field_values_iter(&self, field: &str) -> Option<impl Iterator<Item = &Rc<str>>> {
        self.fields.get(field).map(|v| v.iter())
    }
}
