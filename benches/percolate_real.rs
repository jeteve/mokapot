use std::collections::HashMap;
use std::rc::Rc;

use criterion::{black_box, Throughput};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use fake::faker::address::en::CountryName;
use fake::Fake;
use mokapot::models::percolator::{MultiPercolator, SimplePercolator};
use mokapot::models::queries::ConjunctionQuery;
use mokapot::models::{documents::Document, percolator::Percolator, queries::TermQuery};

const FIELD: &str = "city";
const FIELD2: &str = "taste";
const THIRD_FIELDS: [&str; 4] = ["price", "colour", "size", "temperature"];

static TASTE_VALUES: [&str; 5] = ["acidic", "sweet", "bitter", "umani", "salty"];

fn one_random_data<T: Clone, const N: usize>(d: &[T; N]) -> T {
    d[(0..N).fake::<usize>()].clone()
}

fn one_random_from_vec<T: Clone>(d: &Vec<T>) -> T {
    d[(0..d.len()).fake::<usize>()].clone()
}

fn build_query(_n: usize, third_fields: &HashMap<&str, Vec<String>>) -> ConjunctionQuery {
    let q1 = TermQuery::new(FIELD.into(), CountryName().fake::<String>().into());
    let q1b = TermQuery::new(FIELD2.into(), one_random_data(&TASTE_VALUES).into());
    //let q1 = TermQuery::new(FIELD.into(), format!("value{n}").into());
    // Only 4 values for this one.
    //let q2 = TermQuery::new(FIELD2.into(), format!("value{}", n % 4).into());
    let q3_field = one_random_data(&THIRD_FIELDS);
    let q3_value = one_random_from_vec(third_fields.get(q3_field).unwrap());
    let q3 = TermQuery::new(q3_field.into(), q3_value.into());

    ConjunctionQuery::new(vec![Box::new(q1), Box::new(q1b), Box::new(q3)])
}

fn build_percolator<P>(n: usize, third_fields: &HashMap<&str, Vec<String>>) -> P
where
    P: Percolator + std::fmt::Display + Default,
{
    let mut p = P::default();
    (0..n).map(|n| build_query(n, third_fields)).for_each(|q| {
        p.add_query(Rc::new(q));
    });
    p
}

fn percolate_real(c: &mut Criterion) {
    let mut group = c.benchmark_group("Reallife_matching");

    let third_fields = HashMap::from([
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
            "size",
            vec!["small".into(), "medium".into(), "large".into()],
        ),
        (
            "temperature",
            vec![
                "freezing".into(),
                "cold".into(),
                "tepid".into(),
                "warm".into(),
                "hot".into(),
            ],
        ),
        (
            "price",
            (1..100).map(|p| format!("price_{}", p)).collect::<Vec<_>>(),
        ),
    ]);

    for nqueries in [100, 10000, 50000] {
        group.throughput(Throughput::Elements(1));

        // Build percolators with n queries field=valueN
        let p = build_percolator::<SimplePercolator>(nqueries, &third_fields);
        let mp = build_percolator::<MultiPercolator>(nqueries, &third_fields);

        fn build_document(_n: usize, third_fields: &HashMap<&str, Vec<String>>) -> Document {
            let d = Document::new()
                .with_value(FIELD, CountryName().fake::<String>())
                .with_value(FIELD2, one_random_data(&TASTE_VALUES));
            let q3_field = one_random_data(&THIRD_FIELDS);
            let q3_value = one_random_from_vec(third_fields.get(q3_field).unwrap());
            d.with_value(q3_field, q3_value)
        }

        let input_size = criterion::BatchSize::SmallInput;

        group.bench_function(BenchmarkId::new("perc_dyna", &p), |b| {
            b.iter_batched(
                || build_document(nqueries, &third_fields),
                |d| black_box(p.qids_from_document(&d).next()),
                input_size,
            )
        });

        group.bench_function(BenchmarkId::new("multi_perc", &mp), |b| {
            b.iter_batched(
                || build_document(nqueries, &third_fields),
                |d| black_box(mp.bs_qids_from_document(&d).next()),
                input_size,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, percolate_real);
criterion_main!(benches);
