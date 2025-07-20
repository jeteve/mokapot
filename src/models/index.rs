use std::collections::HashMap;
use std::rc::Rc;

use super::documents::Document;

pub type DocId = usize;

#[derive(Debug, Default)]
pub struct Index {
    // Remember the documents
    documents: Vec<Document>,
    // The inverted indices for each ( field,  value)
    inverted_indices: HashMap<(Rc<str>, Rc<str>), Vec<DocId>>,
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
            .map(|doc_ids| doc_ids.iter().cloned())
            .unwrap_or_default()
    }

    pub fn index_document(&mut self, d: Document) -> DocId {
        // Save the new document
        self.documents.push(d.clone());

        let new_doc_id = self.documents.len() - 1;

        // Update the right inverted indices.
        for field in d.fields() {
            for value in d.field_values_iter(field.as_ref()).unwrap() {
                let value_index = self
                    .inverted_indices
                    .entry((field.clone(), value.clone()))
                    .or_default();
                value_index.push(new_doc_id);
            }
        }
        new_doc_id
    }

    pub fn get_document(&self, doc_id: DocId) -> Option<&Document> {
        self.documents.get(doc_id)
    }

    pub fn get_documents(&self) -> &[Document] {
        &self.documents
    }
}
