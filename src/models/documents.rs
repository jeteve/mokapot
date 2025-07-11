use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Default, Clone)]
pub struct Document {
    // Fields representing the document's content
    fields: HashMap<Rc<str>, Vec<Rc<str>>>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_field(mut self, field: Rc<str>, value: Rc<str>) -> Self {
        self.fields
            .entry(field)
            .and_modify(|v| v.push(value.clone()))
            .or_insert(vec![value]);
        self
    }

    pub fn fields(&self) -> impl Iterator<Item = &Rc<str>> {
        self.fields.keys()
    }

    pub fn field_values_iter(&self, field: &str) -> Option<impl Iterator<Item = &Rc<str>>> {
        self.fields.get(field).map(|v| v.iter())
    }
}
