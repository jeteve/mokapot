use std::collections::HashMap;
use std::rc::Rc;

use roaring::RoaringBitmap;

use super::document::Document;

pub type DocId = u32;

#[derive(Debug, Default)]
pub struct Index {
    // Remember the documents
    //documents: Vec<Document>,
    // The inverted indices for each ( field,  value)
    inverted_idx_bs: HashMap<(Rc<str>, Rc<str>), RoaringBitmap>,
    empty_bs: RoaringBitmap,
    n_documents: DocId,
}

impl Index {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many documents were indexed.
    pub fn len(&self) -> usize {
        self.n_documents as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// An iterator on matching (field,value) docs IDs
    pub fn docs_from_fv_iter<T, U>(&self, field: T, value: U) -> impl Iterator<Item = DocId> + '_
    where
        T: Into<Rc<str>>,
        U: Into<Rc<str>>,
    {
        self.docs_from_fv(field, value).iter()
    }

    /// A RoaringBitmap of doc IDs matching the field value.
    pub fn docs_from_fv<T, U>(&self, field: T, value: U) -> &RoaringBitmap
    where
        T: Into<Rc<str>>,
        U: Into<Rc<str>>,
    {
        self.inverted_idx_bs
            .get(&(field.into(), value.into()))
            .unwrap_or(&self.empty_bs)
    }

    pub fn index_document(&mut self, d: &Document) -> DocId {
        let new_doc_id = self.n_documents;

        self.n_documents = self
            .n_documents
            .checked_add(1)
            .expect("Too many documents. Max is u32::MAX");

        // Update the right inverted indices.
        for (field, value) in d.field_values() {
            self.inverted_idx_bs
                .entry((field, value))
                .or_default()
                .insert(new_doc_id);
        }
        new_doc_id
    }
}
