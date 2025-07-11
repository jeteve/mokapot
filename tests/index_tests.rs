use mokapot::models::documents::Document;
use mokapot::models::index::Index;

#[test]
fn test_basic_index() {
    let mut index = Index::new();
    let d = Document::default();

    let doc_id = index.index_document(d);
    assert_eq!(doc_id, 0);
}

#[test]
fn test_few_docs() {
    let mut index = Index::new();
    let d1 = Document::default().add_field("colour".into(), "blue".into());
    let d2 = Document::default().add_field("colour".into(), "green".into());
    let d3 = Document::default()
        .add_field("taste".into(), "sweet".into())
        .add_field("colour".into(), "blue".into());

    let doc_id1 = index.index_document(d1);
    let doc_id2 = index.index_document(d2);
    let _ = index.index_document(d3);

    assert_eq!(doc_id1, 0);
    assert_eq!(doc_id2, 1);
    assert_eq!(index.get_documents().len(), 3);

    assert!(index.term_iter("shape", "sausage").is_none());
    assert!(index.term_iter("colour", "purple").is_none());
    assert!(index.term_iter("colour", "blue").is_some());
    assert!(index.term_iter("taste", "sweet").is_some());

    let sweet_docs = index
        .term_iter("taste", "sweet")
        .map(|iter| iter.collect::<Vec<_>>());

    assert_eq!(sweet_docs, Some(vec![2]));

    let blue_docs = index
        .term_iter("colour", "blue")
        .map(|iter| iter.collect::<Vec<_>>());
    assert_eq!(blue_docs, Some(vec![0, 2]));
}
