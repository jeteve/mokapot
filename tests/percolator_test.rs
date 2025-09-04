use std::rc::Rc;

use mokapot::models::{
    documents::Document,
    percolator::{Percolator, Qid},
    queries::{ConjunctionQuery, DisjunctionQuery, TermQuery},
};

#[test]
fn test_percolator() {
    let mut mp = Percolator::default();
    let q1 = Rc::new(TermQuery::new("colour".into(), "blue".into()));
    let q1_id = mp.add_query(q1.clone());
    assert_eq!(q1_id, 0);

    let d = Document::new().with_value("colour", "blue");

    let q_ids = mp.qids_from_document(&d).collect::<Vec<Qid>>();
    assert_eq!(mp.bs_qids_from_document(&d).collect::<Vec<_>>(), q_ids);
    assert_eq!(q_ids, vec![0]);

    let q_ids = mp.qids_from_document(&d).collect::<Vec<Qid>>();
    assert_eq!(mp.bs_qids_from_document(&d).collect::<Vec<_>>(), q_ids);
    assert_eq!(q_ids, vec![0]);

    let d = Document::new().with_value("colour", "green");
    assert_eq!(
        mp.qids_from_document(&d).collect::<Vec<Qid>>(),
        Vec::<Qid>::new()
    );
    assert_eq!(
        mp.bs_qids_from_document(&d).collect::<Vec<Qid>>(),
        Vec::<Qid>::new()
    );

    let disj = Rc::new(DisjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "blue".into())),
        Box::new(TermQuery::new("colour".into(), "green".into())),
    ]));

    mp.add_query(disj.clone());

    // The colour=green document will match the disjunction query.
    assert_eq!(mp.qids_from_document(&d).collect::<Vec<Qid>>(), vec![1]);
    assert_eq!(mp.bs_qids_from_document(&d).collect::<Vec<_>>(), vec![1]);

    // Now a simple conjunction query
    // ( blue or green ) AND bitter
    let disj = DisjunctionQuery::new(vec![
        Box::new(TermQuery::new("colour".into(), "blue".into())),
        Box::new(TermQuery::new("colour".into(), "green".into())),
    ]);
    let conj = Rc::new(ConjunctionQuery::new(vec![
        Box::new(disj),
        Box::new(TermQuery::new("taste".into(), "bitter".into())),
    ]));

    let cid = mp.add_query(conj.clone());

    // A document that is green will not match. but generate a failed candidate
    // as the conjunction would have mached, because it just indexes the bitter taste,
    // as this is more specific than the conjunction side.
    assert_eq!(mp.qids_from_document(&d).collect::<Vec<Qid>>(), vec![1]);
    assert_eq!(mp.bs_qids_from_document(&d).collect::<Vec<_>>(), vec![1]);

    // Another document that is bitter and green
    let sprout = Document::new()
        .with_value("colour", "green")
        .with_value("taste", "bitter");

    // This time it also matches the conjunction
    assert_eq!(
        mp.qids_from_document(&sprout).collect::<Vec<Qid>>(),
        vec![1, cid]
    );
    assert_eq!(
        mp.bs_qids_from_document(&sprout).collect::<Vec<_>>(),
        vec![1, cid]
    );
}
