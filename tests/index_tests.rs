use std::rc::Rc;

use mokapot::models::documents::Document;
use mokapot::models::index::Index;

#[test]
fn test_basic_index() {
    let mut index = Index::new();
    let d = Document::default();

    let doc_id = index.index_document(&d);
    assert_eq!(doc_id, 0);
}

#[test]
fn test_few_docs() {
    let colour: Rc<str> = "colour".into();
    let taste: Rc<str> = "taste".into();

    let mut index = Index::new();
    let d1 = Document::default().with_value(colour.clone(), "blue");
    let d2 = Document::default().with_value(colour.clone(), "green");
    let d3 = Document::default()
        .with_value(taste.clone(), "sweet")
        .with_value(colour.clone(), "blue");

    let doc_id1 = index.index_document(&d1);
    let doc_id2 = index.index_document(&d2);
    let _ = index.index_document(&d3);

    assert_eq!(doc_id1, 0);
    assert_eq!(doc_id2, 1);
    assert_eq!(index.get_documents().len(), 3);

    assert!(index
        .term_iter("shape".into(), "sausage".into())
        .next()
        .is_none());
    assert!(index
        .term_iter(colour.clone(), "purple".into())
        .next()
        .is_none());
    assert!(index
        .term_iter(colour.clone(), "blue".into())
        .next()
        .is_some());
    assert!(index
        .term_iter(taste.clone(), "sweet".into())
        .next()
        .is_some());

    let sweet_docs = index
        .term_iter(taste.clone(), "sweet".into())
        .collect::<Vec<_>>();

    assert_eq!(sweet_docs, vec![2]);

    let blue_docs = index
        .term_iter(colour.clone(), "blue".into())
        .collect::<Vec<_>>();
    assert_eq!(blue_docs, vec![0, 2]);
}
