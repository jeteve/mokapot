use crate::models::document::Document;

pub(crate) trait DocMatcher {
    fn matches(&self, d: &Document) -> bool;
}
