use mokapot::models::documents::Document;
use mokapot::models::queries::termdisjunction::TermDisjunction;
use mokapot::models::queries::{ConjunctionQuery, DisjunctionQuery, Query, TermQuery};

#[test]
fn test_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let q = TermQuery::new("colour".into(), "blue".into());
    assert!(q.matches(&d));
    assert_eq!(q.to_document().field_values("colour"), vec!["blue".into()]);

    let q2 = TermQuery::new("colour".into(), "red".into());
    assert!(!q2.matches(&d));

    let q3 = TermQuery::new("another_key".into(), "sausage".into());
    assert!(!q3.matches(&d));

    let q_and_q2 = ConjunctionQuery::new(vec![Box::new(q), Box::new(q2)]);

    // let eq = TermQuery::new("another_key".into(), "sausage".into());
    //let enricher = eq.doc_enrichers();
    // Ok, this drop does not compile. Thanks Rust!
    // drop(eq);
    //let q4 = enricher[0].query;
    //assert!(!q4.matches(&d));

    assert!(!q_and_q2.matches(&d));
}

#[test]
fn test_termdisjunction() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let green_or_bitter = TermDisjunction::new(vec![
        TermQuery::new("colour".into(), "green".into()),
        TermQuery::new("taste".into(), "bitter".into()),
    ]);
    assert!(green_or_bitter.matches(&d));

    let red_or_bitter = TermDisjunction::new(vec![
        TermQuery::new("colour".into(), "red".into()),
        TermQuery::new("taste".into(), "bitter".into()),
    ]);
    assert!(!red_or_bitter.matches(&d));
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
    assert_eq!(
        green_or_bitter.to_document().field_values("colour"),
        vec!["green".into()]
    );
    assert_eq!(
        green_or_bitter.to_document().field_values("taste"),
        vec!["bitter".into()]
    );

    approx::assert_relative_eq!(green_or_bitter.specificity(), 0.5);

    let gob_and_b = ConjunctionQuery::new(vec![
        Box::new(green_or_bitter),
        Box::new(TermQuery::new("colour".into(), "blue".into())),
    ]);

    let gob_and_b_doc = gob_and_b.to_document();
    // The single colour=blue is more specific a priori
    assert_eq!(gob_and_b_doc.field_values("colour"), vec!["blue".into()]);
    assert!(gob_and_b_doc.field_values("taste").is_empty());

    let purple_or_bitter = DisjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "purple".into())),
        Box::new(TermQuery::new("taste".into(), "bitter".into())),
    ]);
    assert!(!purple_or_bitter.matches(&d));
}
