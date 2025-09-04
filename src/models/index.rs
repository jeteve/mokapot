use std::collections::HashMap;
use std::rc::Rc;

use roaring::RoaringBitmap;

use super::documents::Document;

pub type DocId = u32;

#[derive(Debug, Default)]
pub struct Index {
    // Remember the documents
    documents: Vec<Document>,
    // The inverted indices for each ( field,  value)
    inverted_indices: HashMap<(Rc<str>, Rc<str>), Vec<DocId>>,
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
        self.inverted_indices
            .get(&(field, term))
            // Copied will turn the &usize into usize
            .map(|doc_ids| doc_ids.iter().copied())
            .unwrap_or_default()
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
        for field in d.fields() {
            for value in d.field_values_iter(field.as_ref()).unwrap() {
                let value_index = self
                    .inverted_indices
                    .entry((field.clone(), value.clone()))
                    .or_default();
                value_index.push(new_doc_id);

                let value_bs = self
                    .inverted_idx_bs
                    .entry((field.clone(), value.clone()))
                    .or_default();

                value_bs.insert(new_doc_id); // Note this only works with u32s
            }
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
