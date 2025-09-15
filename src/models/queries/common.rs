use crate::models::document::Document;

pub(crate) trait Query {
    fn matches(&self, d: &Document) -> bool;
}
