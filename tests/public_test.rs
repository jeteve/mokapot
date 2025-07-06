use mokapot::models::{Document, TermQuery};

#[test]
fn test_query() {
    let d: Document = Document::default()
        .add_field("colour".to_owned(), "blue".to_owned())
        .add_field("colour".to_owned(), "green".to_owned())
        .add_field("taste".to_owned(), "sweet".to_owned());

    let q = TermQuery::new("colour".to_owned(), "blue".to_owned());
    assert!(q.matches(d));
}
