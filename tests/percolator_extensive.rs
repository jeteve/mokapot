use std::rc::Rc;

use fakeit;
use rand::Rng;

use mokapot::models::documents::Document;
use mokapot::models::percolator::Percolator;
use mokapot::models::queries::{ConjunctionQuery, TermQuery};

#[test]
fn test_percolator() {
    // Generate 1000 random documents.
    let mut rng = rand::rng();
    let mut docs: Vec<Document> = vec![];
    let field_names = vec!["colour", "taste", "shape", "price", "smell"];
    for _ in 0..1000 {
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
    for d in docs {
        total_nres += p.static_qids_from_document(&d).count();
    }

    assert_ne!(total_nres, 0);
}
