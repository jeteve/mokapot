use std::rc::Rc;

use mokapot::models::document::Document;
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
    assert_eq!(index.len(), 3);

    assert!(index
        .docs_from_fv_iter("shape".into(), "sausage".into())
        .next()
        .is_none());

    assert!(index
        .docs_from_fv("shape".into(), "sausage".into())
        .iter()
        .next()
        .is_none());

    assert!(index
        .docs_from_fv_iter(colour.clone(), "purple".into())
        .next()
        .is_none());

    assert!(index
        .docs_from_fv(colour.clone(), "purple".into())
        .iter()
        .next()
        .is_none());

    assert!(index
        .docs_from_fv_iter(colour.clone(), "blue".into())
        .next()
        .is_some());

    assert!(index
        .docs_from_fv(colour.clone(), "blue".into())
        .iter()
        .next()
        .is_some());

    assert!(index
        .docs_from_fv_iter(taste.clone(), "sweet".into())
        .next()
        .is_some());

    assert!(index
        .docs_from_fv(taste.clone(), "sweet".into())
        .iter()
        .next()
        .is_some());

    let sweet_docs = index
        .docs_from_fv_iter(taste.clone(), "sweet".into())
        .collect::<Vec<_>>();

    assert_eq!(sweet_docs, vec![2]);

    let sweet_docs = index
        .docs_from_fv(taste.clone(), "sweet".into())
        .iter()
        .collect::<Vec<_>>();

    assert_eq!(sweet_docs, vec![2]);

    let blue_docs = index
        .docs_from_fv_iter(colour.clone(), "blue".into())
        .collect::<Vec<_>>();
    assert_eq!(blue_docs, vec![0, 2]);

    let blue_docs = index
        .docs_from_fv(colour.clone(), "blue".into())
        .iter()
        .collect::<Vec<_>>();
    assert_eq!(blue_docs, vec![0, 2]);
}
