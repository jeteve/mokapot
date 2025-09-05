use std::collections::HashMap;
use std::rc::Rc;

use roaring::RoaringBitmap;

use super::document::Document;

pub type DocId = u32;

#[derive(Debug, Default)]
pub struct Index {
    // Remember the documents
    documents: Vec<Document>,
    // The inverted indices for each ( field,  value)
    inverted_idx_bs: HashMap<(Rc<str>, Rc<str>), RoaringBitmap>,
    empty_bs: RoaringBitmap,
}

impl Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn term_iter(&self, field: Rc<str>, term: Rc<str>) -> impl Iterator<Item = DocId> + '_ {
        self.term_bs(field, term).iter()
    }

    pub fn term_bs(&self, field: Rc<str>, term: Rc<str>) -> &RoaringBitmap {
        self.inverted_idx_bs
            .get(&(field, term))
            .unwrap_or(&self.empty_bs)
    }

    pub fn index_document(&mut self, d: &Document) -> DocId {
        // Save the new document
        self.documents.push(d.clone());

        let new_doc_id = (self.documents.len() - 1)
            .try_into()
            .expect("Exceeded max size for index");

        // Update the right inverted indices.
        for (field, value) in d.field_values() {
            self.inverted_idx_bs
                .entry((field, value))
                .or_default()
                .insert(new_doc_id);
        }
        new_doc_id
    }

    pub fn get_document(&self, doc_id: DocId) -> Option<&Document> {
        self.documents.get(doc_id as usize)
    }

    pub fn get_documents(&self) -> &[Document] {
        &self.documents
    }
}
