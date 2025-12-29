use mokaccino::{models::cnf::Query, prelude::Percolator};

#[test]
fn test_random_query_strings() {
    let mut rng = rand::rng();
    for _ in 0..1000 {
        let s = Query::random_string(&mut rng);
        assert!(s.parse::<Query>().is_ok());
    }
}

#[test]
// test we can index random queries.
fn test_random_queries() {
    let mut p = Percolator::default();
    let mut rng = rand::rng();
    for _ in 0..1000 {
        let q = Query::random(&mut rng);
        // This should no panic.
        _ = p.add_query(q);
    }
}

#[test]
fn test_random_is_not_default() {
    let mut rng = rand::rng();
    for _ in 0..100 {
        let q = Query::random(&mut rng);
        assert_ne!(q, Query::default());
    }
}
