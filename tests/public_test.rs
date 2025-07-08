use mokapot::models::{ConjunctionQuery, Document, Query, TermQuery};

#[test]
fn test_query() {
    let d: Document = Document::default()
        .add_field("colour".into(), "blue".into())
        .add_field("colour".into(), "green".into())
        .add_field("taste".into(), "sweet".into());

    let q = TermQuery::new("colour".into(), "blue".into());
    assert!(q.matches(&d));

    let q2 = TermQuery::new("colour".into(), "red".into());
    assert!(!q2.matches(&d));

    let q_and_q2 = ConjunctionQuery::new(vec![Box::new(q), Box::new(q2)]);
    assert!(!q_and_q2.matches(&d));

    let green_and_sweet = ConjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "green".into())),
        Box::new(TermQuery::new("taste".into(), "sweet".into())),
    ]);
    assert!(green_and_sweet.matches(&d));
}
