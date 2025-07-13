use mokapot::models::{
    documents::Document,
    index::Index,
    queries::{Query, TermQuery},
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
    index.index_document(d.clone());
    index.index_document(d2.clone());

    let q = TermQuery::new("colour".into(), "blue".into());
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
