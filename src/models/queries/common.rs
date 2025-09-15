use crate::models::document::Document;

pub type PreHeatFn = Box<dyn Fn(Document) -> Document>;

pub trait Query {
    fn matches(&self, d: &Document) -> bool;
}
