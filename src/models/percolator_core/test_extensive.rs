/*

Extensive test for percolating lots of documents against lot of queries.

*/

#[cfg(test)]
use rand::prelude::*;

#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
use fake::Fake;

#[cfg(test)]
use fake::faker::address::en::*;

#[cfg(test)]
use crate::models::cnf::Query;

#[cfg(test)]
use crate::models::document::Document;

#[cfg(test)]
use crate::models::percolator_core::PercolatorCore;

#[cfg(test)]
use crate::models::queries::term::TermQuery;

#[cfg(test)]
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

    let mut p = PercolatorCore::default();

    // Add 5000 documents as a sample
    //for d in docs.iter().take(10000) {
    //    p.add_sample_document(d);
    //}

    // Generate and index 100,000 conjunction queries that operate on
    // different fields please.
    for _ in 0..10000 {
        // The first query is always a random city.

        // WARNING. There will be no difference in efficiency
        // with that. The order of q1/e
        let q1 = TermQuery::new("city", CountryName().fake::<String>());

        let q1b = TermQuery::new("taste", one_random_data(&field_values["taste"]));

        // The last field is random.
        let f2 = one_random_data(&field_names);
        let f_value = one_random_data(field_values.get(f2).unwrap());
        let q2 = TermQuery::new(f2, f_value);

        let mut qs: Vec<Query> = vec![q1, q1b, q2]
            .into_iter()
            .map(Query::from_termquery)
            .collect();
        qs.shuffle(&mut rng);

        let q = Query::from_and(qs);
        //println!("Adding query={}", q.to_cnf());
        let _ = p.safe_add_query(q).unwrap();
    }
}
