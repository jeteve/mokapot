use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use fake::faker::address::en::*;
use fake::Fake;

use mokapot::itertools::*;
use mokapot::models::documents::Document;
use mokapot::models::percolator::{Percolator, Qid};
use mokapot::models::queries::{ConjunctionQuery, Query, TermQuery};

fn one_random_data<T: Clone>(d: &[T]) -> T {
    d[(0..d.len()).fake::<usize>()].clone()
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
    for _ in 0..20000 {
        let mut d = Document::default();
        // We always have one random city.
        d = d.with_value("city", CountryName().fake::<String>());

        // And a colour
        d = d.with_value("colour", one_random_data(&field_values["colour"]));

        // And a taste
        d = d.with_value("taste", one_random_data(&field_values["taste"]));

        // And a price (only 100 price possible)
        d = d.with_value("price", one_random_data(&field_values["price"]));

        docs.push(d);
    }

    let mut p = Percolator::new();

    // Add 5000 documents as a sample
    for d in docs.iter().take(10000) {
        p.add_sample_document(d);
    }

    // Generate and index 100 conjunction queries that operate on
    // different fields please.
    for _ in 0..1000 {
        // The first query is always a random city.

        // WARNING. There will be no difference in efficiency
        // with that. The order of q1/e
        let q1 = TermQuery::new("city".into(), CountryName().fake::<String>().into());

        let q1b = TermQuery::new(
            "taste".into(),
            one_random_data(&field_values["taste"]).into(),
        );

        // The last field is random.
        let f2 = one_random_data(&field_names);
        let f_value = one_random_data(field_values.get(f2).unwrap());
        let q2 = TermQuery::new(f2.into(), f_value.into());

        let mut qs: Vec<Box<dyn Query>> = vec![q1, q1b, q2]
            .into_iter()
            .map(|q| Box::new(q) as Box<dyn Query>)
            .collect();
        qs.shuffle(&mut rng);

        let q = ConjunctionQuery::new(qs);
        // println!("Adding query={:?}", q);
        p.add_query(Rc::new(q));
    }

    let mut total_nres = 0;
    let mut total_pre: usize = 0;
    let mut total_post: usize = 0;
    for d in docs.iter().rev().take(10000) {
        //println!("Percolating {:?}", d);
        let mut res_i = p.qids_from_document(d).with_stat();

        let res = res_i.by_ref().collect::<HashSet<Qid>>();

        // Same sets of Query IDs in both cases
        assert!(res_i.pre_nested() >= res_i.post_nested());
        //println!("Pre/Post: {} {}", res_i.pre_nested(), res_i.post_nested());
        //println!("Matching queries:");

        //for qid in res.iter() {
        //    let q = p.get_query(*qid);
        //println!("{:?}", q);
        //}

        total_pre += res_i.pre_nested();
        total_post += res_i.post_nested();

        total_nres += res.len();
    }

    assert_ne!(total_nres, 0);
    println!("Pre/Post: {} {}", total_pre, total_post);
    println!("Efficiency: {}", total_post as f64 / total_pre as f64)
}
