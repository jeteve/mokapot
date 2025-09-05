use mokapot::models::document::Document;
use mokapot::models::queries::{ConjunctionQuery, DisjunctionQuery, Query, TermQuery};

#[test]
fn test_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let q = TermQuery::new("colour".into(), "blue".into());
    assert!(q.matches(&d));
    assert_eq!(q.to_cnf().to_string(), "(AND (OR colour=blue))");

    let q2 = TermQuery::new("colour".into(), "red".into());
    assert!(!q2.matches(&d));
    assert_eq!(q2.to_cnf().to_string(), "(AND (OR colour=red))");

    let q3 = TermQuery::new("another_key".into(), "sausage".into());
    assert!(!q3.matches(&d));
    assert_eq!(q3.to_cnf().to_string(), "(AND (OR another_key=sausage))");

    let q_and_q2 = ConjunctionQuery::new(vec![Box::new(q), Box::new(q2)]);

    // let eq = TermQuery::new("another_key".into(), "sausage".into());
    //let enricher = eq.doc_enrichers();
    // Ok, this drop does not compile. Thanks Rust!
    // drop(eq);
    //let q4 = enricher[0].query;
    //assert!(!q4.matches(&d));

    assert!(!q_and_q2.matches(&d));
    assert!(q_and_q2.to_cnf().to_string() == "(AND (OR colour=blue) (OR colour=red))");
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
    assert_eq!(
        green_and_sweet.to_cnf().to_string(),
        "(AND (OR colour=green) (OR taste=sweet))"
    );

    let green_or_bitter = DisjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "green".into())),
        Box::new(TermQuery::new("taste".into(), "bitter".into())),
    ]);
    assert!(green_or_bitter.matches(&d));
    assert_eq!(
        green_or_bitter.to_cnf().to_string(),
        "(AND (OR colour=green taste=bitter))"
    );

    // a disjunction of conjunctions
    let blue_and_sweet = ConjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "blue".into())),
        Box::new(TermQuery::new("taste".into(), "sweet".into())),
    ]);
    let green_and_bitter = ConjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "green".into())),
        Box::new(TermQuery::new("taste".into(), "bitter".into())),
    ]);
    let gab_or_bas =
        DisjunctionQuery::new(vec![Box::new(green_and_bitter), Box::new(blue_and_sweet)]);
    assert!(gab_or_bas.matches(&d));
    assert_eq!(
        gab_or_bas.to_cnf().to_string(),
        "(AND (OR colour=blue colour=green) (OR colour=green taste=sweet) (OR colour=blue taste=bitter) (OR taste=bitter taste=sweet))"
    );

    let gob_and_b = ConjunctionQuery::new(vec![
        Box::new(green_or_bitter),
        Box::new(TermQuery::new("colour".into(), "blue".into())),
    ]);

    let purple_or_bitter = DisjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "purple".into())),
        Box::new(TermQuery::new("taste".into(), "bitter".into())),
    ]);
    assert!(!purple_or_bitter.matches(&d));

    // The document to match this query
    let other_d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "bitter");
    assert!(gob_and_b.matches(&other_d));

    // A document that does not match
    let non_matching_d: Document = Document::default()
        .with_value("colour", "yellow")
        .with_value("taste", "bitter");
    assert!(!gob_and_b.matches(&non_matching_d));

    // another matching one, but because it has 2 colours
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green");
    assert!(gob_and_b.matches(&d));

    assert_eq!(
        gob_and_b.to_cnf().to_string(),
        "(AND (OR colour=green taste=bitter) (OR colour=blue))"
    );
}
