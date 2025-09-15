use crate::models::document::Document;

pub trait Query {
    fn matches(&self, d: &Document) -> bool;
}
