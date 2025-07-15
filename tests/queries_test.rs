use mokapot::models::documents::Document;
use mokapot::models::queries::{ConjunctionQuery, DisjunctionQuery, Query, TermQuery};

#[test]
fn test_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let q = TermQuery::new("colour".into(), "blue".into());
    assert!(q.matches(&d));

    let q2 = TermQuery::new("colour".into(), "red".into());
    assert!(!q2.matches(&d));

    let q3 = TermQuery::new("another_key".into(), "sausage".into());
    assert!(!q3.matches(&d));

    let q_and_q2 = ConjunctionQuery::new(vec![Box::new(q), Box::new(q2)]);
    assert!(!q_and_q2.matches(&d));
}

#[test]
fn test_conjunction_disjunction_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let green_and_sweet = ConjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "green".into())),
        Box::new(TermQuery::new("taste".into(), "sweet".into())),
    ]);
    assert!(green_and_sweet.matches(&d));

    let green_or_bitter = DisjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "green".into())),
        Box::new(TermQuery::new("taste".into(), "bitter".into())),
    ]);
    assert!(green_or_bitter.matches(&d));

    let purple_or_bitter = DisjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "purple".into())),
        Box::new(TermQuery::new("taste".into(), "bitter".into())),
    ]);
    assert!(!purple_or_bitter.matches(&d));
}
