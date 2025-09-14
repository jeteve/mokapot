use std::rc::Rc;

use crate::models::document::Document;

pub type PreHeatFn = Box<dyn Fn(Document) -> Document>;

pub struct PreHeater {
    id: Rc<str>,
    preheat: PreHeatFn,
}

impl PreHeater {
    pub fn new<T: Into<Rc<str>>>(id: T, preheat: PreHeatFn) -> Self {
        Self {
            id: id.into(),
            preheat,
        }
    }
}

pub trait Query {
    fn matches(&self, d: &Document) -> bool;
}

mod test {
    use crate::{models::queries::PreHeater, prelude::Document};

    #[test]
    fn test_preheater() {
        let ph = PreHeater::new("identity", Box::new(|d| d));

        assert!((ph.preheat)(Document::default()) == Document::default());
    }
}
