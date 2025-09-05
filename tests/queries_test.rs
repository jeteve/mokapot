use mokapot::models::cnf::*;
use mokapot::models::document::Document;

#[test]
fn test_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let q = "colour".has_value("blue");
    assert!(q.matches(&d));
    assert_eq!(q.to_string(), "(AND (OR colour=blue))");

    let q2 = "colour".has_value("red");
    assert!(!q2.matches(&d));
    assert_eq!(q2.to_string(), "(AND (OR colour=red))");

    let q3 = "another_key".has_value("sausage");
    assert!(!q3.matches(&d));
    assert_eq!(q3.to_string(), "(AND (OR another_key=sausage))");

    let q_and_q2 = q & q2;

    assert!(!q_and_q2.matches(&d));
    assert!(q_and_q2.to_string() == "(AND (OR colour=blue) (OR colour=red))");
}

#[test]
fn test_conjunction_disjunction_query() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let green_and_sweet = "colour".has_value("green") & "taste".has_value("sweet");
    assert!(green_and_sweet.matches(&d));
    assert_eq!(
        green_and_sweet.to_string(),
        "(AND (OR colour=green) (OR taste=sweet))"
    );

    let green_or_bitter = "colour".has_value("green") | "taste".has_value("bitter");
    assert!(green_or_bitter.matches(&d));
    assert_eq!(
        green_or_bitter.to_string(),
        "(AND (OR colour=green taste=bitter))"
    );

    // a disjunction of conjunctions
    let blue_and_sweet = "colour".has_value("blue") & "taste".has_value("sweet");
    let green_and_bitter = "colour".has_value("green") & "taste".has_value("bitter");
    let gab_or_bas = green_and_bitter | blue_and_sweet;
    assert!(gab_or_bas.matches(&d));
    assert_eq!(
        gab_or_bas.to_string(),
        "(AND (OR colour=blue colour=green) (OR colour=green taste=sweet) (OR colour=blue taste=bitter) (OR taste=bitter taste=sweet))"
    );

    let gob_and_b = green_or_bitter & "colour".has_value("blue");

    let purple_or_bitter = "colour".has_value("purple") | "taste".has_value("bitter");
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
        gob_and_b.to_string(),
        "(AND (OR colour=green taste=bitter) (OR colour=blue))"
    );
}
