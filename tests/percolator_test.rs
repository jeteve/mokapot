use mokaccino::models::{cnf::*, document::Document, percolator::Percolator, percolator_core::Qid};
use num_traits::Zero;

#[test]
fn test_percolator_vanilla() {
    let mut mp = Percolator::default();
    let q1 = "colour".has_value("blue");
    let q1_id = mp.add_query(q1);
    assert_eq!(q1_id, 0);
    assert!(!mp.to_string().is_empty());
    assert!(!mp.get_query(q1_id).to_string().is_empty());
    assert_eq!(Some(mp.get_query(q1_id)), mp.safe_get_query(q1_id));

    let d = Document::new().with_value("colour", "blue");

    assert_eq!(mp.percolate(&d).collect::<Vec<_>>(), vec![0]);

    let d = Document::new().with_value("colour", "green");

    // We get the empty vector
    assert_eq!(mp.percolate(&d).collect::<Vec<Qid>>(), Vec::<Qid>::new());

    assert!(!mp.stats().to_string().is_empty());
}

#[test]
fn test_percolator_core() {
    let mut mp = Percolator::builder()
        .prefix_sizes(vec![1, 2, 5, 8, 13])
        .build();
    let q1 = "colour".has_value("blue");
    let q1_id = mp.add_query(q1);
    assert_eq!(q1_id, 0);
    assert!(!mp.to_string().is_empty());
    assert!(!mp.get_query(q1_id).to_string().is_empty());
    assert_eq!(Some(mp.get_query(q1_id)), mp.safe_get_query(q1_id));

    let d = Document::new().with_value("colour", "blue");

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

    assert!(!mp.stats().to_string().is_empty());

    let stats = mp.stats();
    assert!(stats.clauses_per_query().mean() > 0.0);
    assert!(stats.preheaters_per_query().mean().is_zero());
    assert_eq!(stats.n_preheaters(), 0);
    assert_eq!(stats.n_queries(), 3);
}
