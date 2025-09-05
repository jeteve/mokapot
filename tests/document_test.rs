use mokapot::models::document::Document;

#[test]
fn test_document_merge() {
    let d1 = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "bitter");
    let d2 = Document::default()
        .with_value("colour", "beige")
        .with_value("colour", "blue");

    let d3 = d1.merge_with(&d2);

    assert_eq!(d3.values("size").len(), 0);
    assert_eq!(d3.values("colour"), vec!["blue".into(), "beige".into()]);
    assert_eq!(d3.values("taste"), vec!["bitter".into()]);
}

#[test]
fn test_document_to_clause() {
    let d = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "bitter")
        .with_value("taste", "sweet");

    let clause = d.to_clause();
    assert_eq!(
        clause.to_string(),
        "(OR colour=blue taste=bitter taste=sweet)"
    );

    let d = Document::default();
    assert_eq!(d.to_clause().to_string(), "(OR )");
}
