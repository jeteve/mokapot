use mokapot::models::documents::Document;

#[test]
fn test_document_merge() {
    let d1 = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "bitter");
    let d2 = Document::default()
        .with_value("colour", "beige")
        .with_value("colour", "blue");

    let d3 = d1.merge_with(&d2);

    assert_eq!(d3.field_values("size").len(), 0);
    assert_eq!(d3.field_values("colour"), vec!["blue", "beige"]);
    assert_eq!(d3.field_values("taste"), vec!["bitter"]);
}
