use mokaccino::models::{
    cnf::*,
    document::Document,
    percolator::{Percolator, Qid},
};

#[test]
fn test_percolator() {
    let mut mp = Percolator::default();
    let q1 = "colour".has_value("blue");
    let q1_id = mp.add_query(q1);
    assert_eq!(q1_id, 0);

    let d = Document::new().with_value("colour", "blue");

    assert_eq!(mp.percolate(&d).collect::<Vec<_>>(), vec![0]);

    assert_eq!(mp.percolate(&d).collect::<Vec<_>>(), vec![0]);

    let d = Document::new().with_value("colour", "green");

    assert_eq!(mp.percolate(&d).collect::<Vec<Qid>>(), Vec::<Qid>::new());

    let disj = "colour".has_value("blue") | "colour".has_value("green");

    mp.add_query(disj.clone());

    // The colour=green document will match the disjunction query.
    assert_eq!(mp.percolate(&d).collect::<Vec<_>>(), vec![1]);

    // Now a simple conjunction query
    // ( blue or green ) AND bitter
    let conj =
        ("colour".has_value("green") | "colour".has_value("blue")) & "taste".has_value("bitter");
    let cid = mp.add_query(conj.clone());

    // A document that is green will not match. but generate a failed candidate
    // as the conjunction would have mached, because it just indexes the bitter taste,
    // as this is more specific than the conjunction side.
    assert_eq!(mp.percolate(&d).collect::<Vec<_>>(), vec![1]);

    // Another document that is bitter and green
    let sprout = Document::new()
        .with_value("colour", "green")
        .with_value("taste", "bitter");

    // This time it also matches the conjunction
    assert_eq!(mp.percolate(&sprout).collect::<Vec<_>>(), vec![1, cid]);
}
