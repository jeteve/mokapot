/*

Extensive test for percolating lots of documents against lot of queries.

Using Simple Percolator with sampling:

Skipped: 130145, Matched: 1154, Churn per match: 112.77729636048527

Using plain MultiPercolator:

Skipped: 0, Matched: 1145, Churn per match: 0


*/

use rand::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;

use fake::faker::address::en::*;
use fake::Fake;

use mokapot::models::documents::Document;
use mokapot::models::percolator::{MultiPercolator, Percolator};
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

    let mut p = MultiPercolator::default();

    // Add 5000 documents as a sample
    //for d in docs.iter().take(10000) {
    //    p.add_sample_document(d);
    //}

    // Generate and index 100,000 conjunction queries that operate on
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
        //println!("Adding query={}", q.to_cnf());
        p.add_query(Rc::new(q));
    }

    let mut tot_skipped = 0;
    let mut tot_matched = 0;

    for d in docs.iter().rev().take(2000) {
        //println!("Percolating {:?}", d);
        let res_i = p.tracked_qids_from_document(d);

        let res = res_i.collect::<Vec<_>>();

        // Same sets of Query IDs in both cases
        //println!("Matching queries:");

        for tqid in res.iter() {
            tot_skipped += tqid.n_skipped();
            tot_matched += 1;
            let q = p.get_query(tqid.qid);
            //println!("{:?}", q);
        }
    }

    assert!(tot_matched > 0);
    println!(
        "Skipped: {}, Matched: {}, Churn per match: {}",
        tot_skipped,
        tot_matched,
        tot_skipped as f64 / tot_matched as f64,
    );
}
