use std::collections::HashSet;

use mokapot::models::{
    cnf::Clause,
    document::Document,
    index::{DocId, Index},
    percolator::clause_docs_from_idx,
    queries::TermQuery,
};

#[test]
fn test_clause_match() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("colour", "green")
        .with_value("taste", "sweet");

    let green_or_bitter = Clause::from_termqueries(vec![
        TermQuery::new("colour", "green"),
        TermQuery::new("taste", "bitter"),
    ]);
    assert!(green_or_bitter.matches(&d));

    let red_or_bitter = Clause::from_termqueries(vec![
        TermQuery::new("colour", "red"),
        TermQuery::new("taste", "bitter"),
    ]);
    assert!(!red_or_bitter.matches(&d));
}

#[test]
fn test_clause() {
    let d: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "sweet");

    let d1: Document = Document::default()
        .with_value("colour", "yellow")
        .with_value("taste", "sour");

    let d2: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "bitter");

    let d3: Document = Document::default()
        .with_value("colour", "blue")
        .with_value("taste", "sweet");

    let d4: Document = Document::default()
        .with_value("colour", "yellow")
        .with_value("taste", "bitter");

    let one_clause = Clause::from_termqueries(vec![TermQuery::new("colour", "blue")]);
    assert!(one_clause.matches(&d));

    let mut index = Index::new();
    // Query against the empty index.

    let doc_ids: Vec<_> = clause_docs_from_idx(&one_clause, &index).iter().collect();
    assert_eq!(doc_ids, vec![]);

    let q = TermQuery::new("colour", "blue");
    let q2 = TermQuery::new("taste", "sweet");
    let disq = Clause::from_termqueries(vec![q, q2]);

    assert!(disq.matches(&d));

    let doc_ids: Vec<_> = clause_docs_from_idx(&disq, &index).iter().collect();
    assert_eq!(doc_ids, vec![]);

    index.index_document(&d);
    index.index_document(&d1);
    index.index_document(&d2);
    index.index_document(&d3);
    index.index_document(&d4);

    // colour = blue or taste = sweet.
    let doc_ids: HashSet<DocId> = clause_docs_from_idx(&disq, &index).iter().collect();
    // Notice the order does not matter..
    assert_eq!(doc_ids, HashSet::from([0, 2, 3]));

    // Test the one term disjunction, to check the
    // optmimisation
    let doc_ids: HashSet<DocId> = clause_docs_from_idx(&one_clause, &index).iter().collect();
    assert_eq!(doc_ids, HashSet::from([0, 2, 3]));
}
