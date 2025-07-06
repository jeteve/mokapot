use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct Document {
    // Fields representing the document's content
    fields: HashMap<String, Vec<String>>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_field(mut self, field: String, value: String) -> Self {
        self.fields
            .entry(field)
            .and_modify(|v| v.push(value.clone()))
            .or_insert(vec![value]);
        self
    }

    pub fn get_field_iter(&self, field: &str) -> Option<impl Iterator<Item = &String>> {
        self.fields.get(field).map(|v| v.iter())
    }
}

pub struct TermQuery {
    field: String,
    term: String,
}

impl TermQuery {
    pub fn new(field: String, term: String) -> Self {
        TermQuery { field, term }
    }

    pub fn matches(&self, d: Document) -> bool {
        d.get_field_iter(&self.field)
            .map_or(false, |mut iter| iter.any(|value| value == &self.term))
    }
}
