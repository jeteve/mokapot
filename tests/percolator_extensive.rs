use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use fake::{Fake, Faker};

use mokapot::itertools::*;
use mokapot::models::documents::Document;
use mokapot::models::percolator::{Percolator, Qid};
use mokapot::models::queries::{ConjunctionQuery, TermQuery};

fn one_random_data<T: Clone>(d: &[T]) -> T {
    d[(0..d.len()).fake::<usize>()].clone()
}

fn two_random_data<T: Clone>(d: &[T]) -> (T, T) {
    let len = d.len();
    let n: usize = (0..len).fake();
    let mut n2: usize = (0..len).fake();
    while n == n2 {
        n2 = (0..len).fake();
    }
    (d[n].clone(), d[n2].clone())
}

#[test]
fn test_percolator() {
    // Generate 10000 random documents.
    let mut rng = rand::rng();
    let mut docs: Vec<Document> = vec![];
    let field_names = vec!["taste", "shape", "price", "colour"];
    let field_values = HashMap::from([
        (
            "colour",
            vec![
                "blue".to_string(),
                "blue".to_string(),
                "green".to_string(),
                "yellow".to_string(),
            ],
        ),
        (
            "taste",
            vec!["bitter", "sweet", "umami", "salty", "acidic"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>(),
        ),
        (
            "shape",
            vec!["circular", "triangular", "square"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>(),
        ),
        (
            "price",
            (1..100).map(|p| format!("price_{}", p)).collect::<Vec<_>>(),
        ),
    ]);
    for _ in 0..10000 {
        let mut d = Document::default();
        // Maybe some fields will be duplicated..
        for _ in 0..rng.random_range(1..field_names.len() * 5) {
            let f_name = one_random_data(&field_names);
            d = d.with_value(f_name, one_random_data(field_values.get(f_name).unwrap()));
        }
        docs.push(d);
    }

    let mut p = Percolator::new();

    // Add 1000 documents as a sample
    for d in docs.iter().take(1000) {
        p.add_sample_document(d);
    }

    // Generate and index 1000 conjunction queries that operate on
    // different fields please.
    for _ in 0..20 {
        let (f1, f2) = two_random_data(&field_names);

        println!("Query fields={} {}", f1, f2);

        let f_value = one_random_data(field_values.get(f1).unwrap());
        let q1 = TermQuery::new(f1.into(), f_value.clone().into());

        let f_value = one_random_data(field_values.get(f2).unwrap());
        let q2 = TermQuery::new(f2.into(), f_value.into());

        let q = ConjunctionQuery::new(vec![Box::new(q1), Box::new(q2)]);
        println!("Adding query={:?}", q);
        p.add_query(Rc::new(q));
    }

    let mut total_nres = 0;
    let mut total_pre: usize = 0;
    let mut total_post: usize = 0;
    for d in docs.iter().rev().take(10) {
        //println!("Percolating {:?}", d);
        let mut res_i = p.static_qids_from_document(d).with_stat();

        let res_static = res_i.by_ref().collect::<HashSet<Qid>>();

        let res_dyn = p.qids_from_document(d).collect::<HashSet<Qid>>();

        // Same sets of Query IDs in both cases
        assert_eq!(res_dyn, res_static);
        assert!(res_i.pre_nested() >= res_i.post_nested());
        //println!("Pre/Post: {} {}", res_i.pre_nested(), res_i.post_nested());
        //println!("Matching queries:");
        for qid in res_static.iter() {
            let q = p.get_query(*qid);
            println!("{:?}", q);
        }

        total_pre += res_i.pre_nested();
        total_post += res_i.post_nested();

        total_nres += res_static.len();
    }

    assert_ne!(total_nres, 0);
    println!("Pre/Post: {} {}", total_pre, total_post);
    println!("Efficiency: {}", total_post as f64 / total_pre as f64)
}
