use bitvec::vec::BitVec;
use std::cmp::max;
use std::collections::HashMap;
use std::rc::Rc;

use super::documents::Document;

type DocId = usize;

const GROWTH_MULTIPLIER: usize = 2;

/// Helper function to set a bit in a BitVec at a specific index.
/// If the index is out of bounds, it will resize the BitVec to accommodate the new index.
/// The BitVec will be resized to double its current size, or to the index length plus 1
/// whatever is larger, if the index exceeds its length
fn set_bitvec_index(bitvec: &mut BitVec, index: usize, value: bool) {
    if index >= bitvec.len() {
        bitvec.resize(max(index, bitvec.len() * GROWTH_MULTIPLIER) + 1, false);
    }
    bitvec.set(index, value);
}

#[derive(Debug, Default)]
pub struct Index {
    // Remember the documents
    documents: Vec<Document>,
    // The inverted indices for each field / value
    inverted_indices: HashMap<Rc<str>, HashMap<Rc<str>, BitVec>>,
}

impl Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn term_iter(&self, field: &str, term: &str) -> Option<impl Iterator<Item = DocId> + '_> {
        self.inverted_indices
            .get(field)
            .and_then(|field_entry| field_entry.get(term))
            .map(|bitvec| bitvec.iter_ones().map(|i| i as DocId))
    }

    pub fn index_document(&mut self, d: Document) -> DocId {
        // Save the new document
        self.documents.push(d.clone());

        let new_doc_id = self.documents.len() - 1;

        // Update the right inverted indices.
        for field in d.fields() {
            let field_entry = self.inverted_indices.entry(field.clone()).or_default();

            for value in d.field_values_iter(field).unwrap() {
                let value_bitvec = field_entry.entry(value.clone()).or_default();
                set_bitvec_index(value_bitvec, new_doc_id, true);
            }
        }
        new_doc_id
    }

    pub fn get_documents(&self) -> &[Document] {
        &self.documents
    }
}

#[cfg(test)]
mod test {
    use super::set_bitvec_index;
    use super::BitVec;

    #[test]
    fn test_set_bitvec_index() {
        let mut bitvec = BitVec::new();
        set_bitvec_index(&mut bitvec, 0, true);
        assert_eq!(bitvec.len(), 1);
        set_bitvec_index(&mut bitvec, 5, true);
        assert!(bitvec[0]);
        assert!(!bitvec[2]);
        assert!(bitvec[5]);
        assert_eq!(bitvec.len(), 6);

        set_bitvec_index(&mut bitvec, 2, true);
        assert!(bitvec[2]);

        set_bitvec_index(&mut bitvec, 10000, true);
        assert!(!bitvec[1000]);
        assert!(bitvec[10000]);
    }
}
