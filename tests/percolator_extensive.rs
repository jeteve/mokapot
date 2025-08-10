use std::collections::HashSet;
use std::rc::Rc;

use fakeit;
use rand::Rng;

use mokapot::itertools::*;
use mokapot::models::documents::Document;
use mokapot::models::percolator::{Percolator, Qid};
use mokapot::models::queries::{ConjunctionQuery, TermQuery};

#[test]
fn test_percolator() {
    // Generate 10000 random documents.
    let mut rng = rand::rng();
    let mut docs: Vec<Document> = vec![];
    let field_names = vec!["colour", "taste", "shape", "price", "smell"];
    for _ in 0..10000 {
        let mut d = Document::default();
        for _ in 0..rng.random_range(0..40) {
            d = d.with_value(
                fakeit::misc::random_data(&field_names),
                fakeit::words::word(),
            );
        }
        docs.push(d);
    }

    let mut p = Percolator::new();
    // Generate 1000 conjunction queries.
    for _ in 0..1000 {
        let q1 = TermQuery::new(
            fakeit::misc::random_data(&field_names).into(),
            fakeit::words::word().into(),
        );
        let q2 = TermQuery::new(
            fakeit::misc::random_data(&field_names).into(),
            fakeit::words::word().into(),
        );

        let q = ConjunctionQuery::new(vec![Box::new(q1), Box::new(q2)]);
        p.add_query(Rc::new(q));
    }

    let mut total_nres = 0;
    let mut total_pre: usize = 0;
    let mut total_post: usize = 0;
    for d in docs {
        let mut res_i = p.static_qids_from_document(&d).with_stat();

        let res_static = res_i.by_ref().collect::<HashSet<Qid>>();

        let res_dyn = p.qids_from_document(&d).collect::<HashSet<Qid>>();

        // Same sets of Query IDs in both cases
        assert_eq!(res_dyn, res_static);
        assert!(res_i.pre_nested() >= res_i.post_nested());
        //println!("Pre/Post: {} {}", res_i.pre_nested(), res_i.post_nested());
        total_pre += res_i.pre_nested();
        total_post += res_i.post_nested();

        total_nres += res_static.len();
    }

    assert_ne!(total_nres, 0);
    println!("Pre/Post: {} {}", total_pre, total_post);
    println!("Efficiency: {}", total_post as f64 / total_pre as f64)
}
