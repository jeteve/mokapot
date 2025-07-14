use mokapot::models::{
    documents::Document,
    index::Index,
    queries::{ConjunctionQuery, DisjunctionQuery, Query, TermQuery},
};

#[test]
fn test_term_query() {
    let d: Document = Document::default()
        .add_field("colour".into(), "blue".into())
        .add_field("colour".into(), "green".into())
        .add_field("taste".into(), "sweet".into());

    let d2: Document = Document::default()
        .add_field("colour".into(), "yellow".into())
        .add_field("colour".into(), "green".into())
        .add_field("taste".into(), "bitter".into());

    let mut index = Index::new();
    // A query on an empty index.
    let q = TermQuery::new("colour".into(), "blue".into());
    assert_eq!(q.docids_from_index(&index).count(), 0);

    index.index_document(d.clone());
    index.index_document(d2.clone());

    assert!(q.matches(&d));
    assert!(q.docids_from_index(&index).next().is_some());
    assert_eq!(q.docids_from_index(&index).count(), 1);

    let q2 = TermQuery::new("colour".into(), "green".into());
    assert!(q2.matches(&d));
    assert!(q2.matches(&d2));
    assert_eq!(q2.docs_from_index(&index).count(), 2);

    let q2 = TermQuery::new("colour".into(), "red".into());
    assert!(!q2.matches(&d));
    assert!(q2.docids_from_index(&index).next().is_none());
    assert_eq!(q2.docids_from_index(&index).count(), 0);

    let q3 = TermQuery::new("another_key".into(), "sausage".into());
    assert!(!q3.matches(&d));
    assert!(q3.docids_from_index(&index).next().is_none());
}

#[test]
fn test_conjunction_query() {
    let d: Document = Document::default()
        .add_field("colour".into(), "blue".into())
        .add_field("taste".into(), "sweet".into());

    let d1: Document = Document::default()
        .add_field("colour".into(), "yellow".into())
        .add_field("taste".into(), "sweet".into());

    let d2: Document = Document::default()
        .add_field("colour".into(), "blue".into())
        .add_field("taste".into(), "bitter".into());

    let d3: Document = Document::default()
        .add_field("colour".into(), "blue".into())
        .add_field("taste".into(), "sweet".into());

    let q = TermQuery::new("colour".into(), "blue".into());
    let q2 = TermQuery::new("taste".into(), "sweet".into());
    let conjunction_query = ConjunctionQuery::new(vec![Box::new(q), Box::new(q2)]);

    assert!(conjunction_query.matches(&d));

    // Index the document
    let mut index = Index::new();
    let doc_ids: Vec<_> = conjunction_query.docids_from_index(&index).collect();
    assert_eq!(doc_ids, vec![]);

    index.index_document(d.clone());
    index.index_document(d1.clone());
    index.index_document(d2.clone());
    index.index_document(d3.clone());

    let mut doc_ids = conjunction_query.docids_from_index(&index);
    assert_eq!(doc_ids.next(), Some(0));
    assert_eq!(doc_ids.next(), Some(3));
    assert_eq!(doc_ids.next(), None);
    assert_eq!(doc_ids.next(), None);
}

#[test]
fn test_disjunction_query() {
    let d: Document = Document::default()
        .add_field("colour".into(), "blue".into())
        .add_field("taste".into(), "sweet".into());

    let d1: Document = Document::default()
        .add_field("colour".into(), "yellow".into())
        .add_field("taste".into(), "sour".into());

    let d2: Document = Document::default()
        .add_field("colour".into(), "blue".into())
        .add_field("taste".into(), "bitter".into());

    let d3: Document = Document::default()
        .add_field("colour".into(), "blue".into())
        .add_field("taste".into(), "sweet".into());

    let d4: Document = Document::default()
        .add_field("colour".into(), "yellow".into())
        .add_field("taste".into(), "bitter".into());

    let q = TermQuery::new("colour".into(), "blue".into());
    let q2 = TermQuery::new("taste".into(), "sweet".into());
    let disq = DisjunctionQuery::new(vec![Box::new(q), Box::new(q2)]);

    assert!(disq.matches(&d));

    let mut index = Index::new();
    // Query against the empty index.
    let doc_ids: Vec<_> = disq.docids_from_index(&index).collect();
    assert_eq!(doc_ids, vec![]);

    index.index_document(d.clone());
    index.index_document(d1.clone());
    index.index_document(d2.clone());
    index.index_document(d3.clone());
    index.index_document(d4.clone());

    // colour = blue or taste = sweet.
    let mut doc_ids = disq.docids_from_index(&index);
    assert_eq!(doc_ids.next(), Some(0));
    assert_eq!(doc_ids.next(), Some(2));
    assert_eq!(doc_ids.next(), Some(3));
    // No more matches!
    assert_eq!(doc_ids.next(), None);
    assert_eq!(doc_ids.next(), None);
}
