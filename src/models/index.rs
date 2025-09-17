//use std::collections::HashMap;
use hashbrown::HashMap;
use std::rc::Rc;

use roaring::RoaringBitmap;

use super::document::Document;

pub type DocId = u32;

#[derive(Debug, Default)]
pub(crate) struct Index {
    // Remember the documents
    //documents: Vec<Document>,
    // The inverted indices for each ( field,  value)
    term_idxs: HashMap<(Rc<str>, Rc<str>), RoaringBitmap>,
    empty_bs: RoaringBitmap,
    n_documents: DocId,
}

impl Index {
    /// How many documents were indexed.
    pub(crate) fn len(&self) -> usize {
        self.n_documents as usize
    }

    /// A RoaringBitmap of doc IDs matching the field value.
    pub(crate) fn docs_from_fv<T, U>(&self, field: T, value: U) -> &RoaringBitmap
    where
        T: Into<Rc<str>>,
        U: Into<Rc<str>>,
    {
        self.term_idxs
            .get(&(field.into(), value.into()))
            .unwrap_or(&self.empty_bs)
    }

    pub(crate) fn index_document(&mut self, d: &Document) -> DocId {
        let new_doc_id = self.n_documents;

        self.n_documents = self
            .n_documents
            .checked_add(1)
            .expect("Too many documents. Max is u32::MAX");

        // Update the right inverted indices.
        for (field, value) in d.field_values() {
            self.term_idxs
                .entry((field, value))
                .or_default()
                .insert(new_doc_id);
        }
        new_doc_id
    }
}

mod test {

    #[test]
    fn test_basic_index() {
        use super::*;

        let mut index: Index = Default::default();
        let d: Document = Default::default();

        let doc_id = index.index_document(&d);
        assert_eq!(doc_id, 0);
    }

    #[test]
    fn test_few_docs() {
        use super::*;
        use std::rc::Rc;
        let colour: Rc<str> = "colour".into();
        let taste: Rc<str> = "taste".into();

        let mut index: Index = Default::default();
        let d1: Document = Document::default().with_value(colour.clone(), "blue");
        let d2 = Document::default().with_value(colour.clone(), "green");
        let d3 = Document::default()
            .with_value(taste.clone(), "sweet")
            .with_value(colour.clone(), "blue");

        let doc_id1 = index.index_document(&d1);
        let doc_id2 = index.index_document(&d2);
        let _ = index.index_document(&d3);

        assert_eq!(doc_id1, 0);
        assert_eq!(doc_id2, 1);
        assert_eq!(index.len(), 3);

        assert!(
            index
                .docs_from_fv("shape", "sausage")
                .iter()
                .next()
                .is_none()
        );

        assert!(
            index
                .docs_from_fv("shape", "sausage")
                .iter()
                .next()
                .is_none()
        );

        assert!(
            index
                .docs_from_fv(colour.clone(), "purple")
                .iter()
                .next()
                .is_none()
        );

        assert!(
            index
                .docs_from_fv(colour.clone(), "purple")
                .iter()
                .next()
                .is_none()
        );

        assert!(
            index
                .docs_from_fv(colour.clone(), "blue")
                .iter()
                .next()
                .is_some()
        );

        assert!(
            index
                .docs_from_fv(colour.clone(), "blue")
                .iter()
                .next()
                .is_some()
        );

        assert!(
            index
                .docs_from_fv(taste.clone(), "sweet")
                .iter()
                .next()
                .is_some()
        );

        assert!(
            index
                .docs_from_fv(taste.clone(), "sweet")
                .iter()
                .next()
                .is_some()
        );

        let sweet_docs = index
            .docs_from_fv(taste.clone(), "sweet")
            .iter()
            .collect::<Vec<_>>();

        assert_eq!(sweet_docs, vec![2]);

        let sweet_docs = index
            .docs_from_fv(taste.clone(), "sweet")
            .iter()
            .collect::<Vec<_>>();

        assert_eq!(sweet_docs, vec![2]);

        let blue_docs = index
            .docs_from_fv(colour.clone(), "blue")
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(blue_docs, vec![0, 2]);

        let blue_docs = index
            .docs_from_fv(colour.clone(), "blue")
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(blue_docs, vec![0, 2]);
    }
}
