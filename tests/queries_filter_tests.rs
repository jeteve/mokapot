use std::rc::Rc;

use mokapot::models::{
    cnf::*, document::Document, index::DocId, index::Index, queries::Query, queries::TermQuery,
};

#[test]
fn test_term_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let d2: Document = Document::default()
        .with_value("colour", "yellow")
        .with_value("colour", "green")
        .with_value("taste", "bitter");

    let mut index = Index::new();
    // A query on an empty index.
    let q = "colour".has_value("blue");
    assert_eq!(q.docs_from_idx_iter(&index).count(), 0);

    index.index_document(&d);
    index.index_document(&d2);

    assert!(q.matches(&d));
    assert!(q.docs_from_idx_iter(&index).next().is_some());
    assert_eq!(q.docs_from_idx_iter(&index).count(), 1);

    let colour: Rc<str> = "colour".into();

    let q2 = TermQuery::new(colour, "green");
    assert!(q2.matches(&d));
    assert!(q2.matches(&d2));
    assert_eq!(q2.docs_from_idx_iter(&index).count(), 2);

    let q2 = TermQuery::new("colour", "red");
    assert!(!q2.matches(&d));
    assert!(q2.docs_from_idx_iter(&index).next().is_none());
    assert_eq!(q2.docs_from_idx_iter(&index).count(), 0);

    let q3 = TermQuery::new("another_key", "sausage");
    assert!(!q3.matches(&d));
    assert!(q3.docs_from_idx_iter(&index).next().is_none());
}

#[test]
fn test_conjunction_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "sweet");

    let d1: Document = Document::default()
        .with_value("colour", "yellow")
        .with_value("taste", "sweet");

    let d2: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "bitter");

    let d3: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "sweet");

    let q = "colour".has_value("blue");
    let q2 = "taste".has_value("sweet");
    let conjunction_query = q & q2;

    assert!(conjunction_query.matches(&d));

    // Index the document
    let mut index = Index::new();
    let doc_ids: Vec<DocId> = conjunction_query.docs_from_idx_iter(&index).collect();
    assert_eq!(doc_ids, vec![] as Vec<DocId>);

    index.index_document(&d);
    index.index_document(&d1);
    index.index_document(&d2);
    index.index_document(&d3);

    let mut doc_ids = conjunction_query.docs_from_idx_iter(&index);
    assert_eq!(doc_ids.next(), Some(0));
    assert_eq!(doc_ids.next(), Some(3));
    assert_eq!(doc_ids.next(), None);
    assert_eq!(doc_ids.next(), None);
}

#[test]
fn test_disjunction_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "sweet");

    let d1: Document = Document::default()
        .with_value("colour", "yellow")
        .with_value("taste", "sour");

    let d2: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "bitter");

    let d3: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "sweet");

    let d4: Document = Document::default()
        .with_value("colour", "yellow")
        .with_value("taste", "bitter");

    let q = "colour".has_value("blue");
    let q2 = "taste".has_value("sweet");
    let disq = q | q2;
    assert!(disq.matches(&d));

    let mut index = Index::new();
    // Query against the empty index.
    let doc_ids: Vec<_> = disq.docs_from_idx_iter(&index).collect();
    assert_eq!(doc_ids, vec![]);

    index.index_document(&d);
    index.index_document(&d1);
    index.index_document(&d2);
    index.index_document(&d3);
    index.index_document(&d4);

    // colour = blue or taste = sweet.
    let mut doc_ids = disq.docs_from_idx_iter(&index);
    assert_eq!(doc_ids.next(), Some(0));
    assert_eq!(doc_ids.next(), Some(2));
    assert_eq!(doc_ids.next(), Some(3));
    // No more matches!
    assert_eq!(doc_ids.next(), None);
    assert_eq!(doc_ids.next(), None);
}
