use std::collections::HashMap;
use std::rc::Rc;

use criterion::{black_box, Throughput};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use fake::faker::address::en::CountryName;
use fake::Fake;
use mokapot::models::percolator::Percolator;
use mokapot::models::queries::ConjunctionQuery;
use mokapot::models::{document::Document, queries::TermQuery};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const FIELD: &str = "city";
const FIELD2: &str = "taste";
const THIRD_FIELDS: [&str; 4] = ["price", "colour", "size", "temperature"];

static TASTE_VALUES: [&str; 5] = ["acidic", "sweet", "bitter", "umani", "salty"];

fn one_random_data<T: Clone, R: Rng + ?Sized, const N: usize>(d: &[T; N], rng: &mut R) -> T {
    d[(0..N).fake_with_rng::<usize, _>(rng)].clone()
}

fn one_random_from_vec<T: Clone, R: Rng + ?Sized>(d: &[T], rng: &mut R) -> T {
    d[(0..d.len()).fake_with_rng::<usize, _>(rng)].clone()
}

fn build_query<R: Rng + ?Sized>(
    _n: usize,
    third_fields: &HashMap<&str, Vec<String>>,
    rng: &mut R,
) -> ConjunctionQuery {
    let q1 = TermQuery::new(
        FIELD.into(),
        CountryName().fake_with_rng::<String, _>(rng).into(),
    );
    let q1b = TermQuery::new(FIELD2.into(), one_random_data(&TASTE_VALUES, rng).into());
    //let q1 = TermQuery::new(FIELD.into(), format!("value{n}").into());
    // Only 4 values for this one.
    //let q2 = TermQuery::new(FIELD2.into(), format!("value{}", n % 4).into());
    let q3_field = one_random_data(&THIRD_FIELDS, rng);
    let q3_value = one_random_from_vec(third_fields.get(q3_field).unwrap(), rng);
    let q3 = TermQuery::new(q3_field.into(), q3_value.into());

    ConjunctionQuery::new(vec![Box::new(q1), Box::new(q1b), Box::new(q3)])
}

fn build_percolator<R: Rng + ?Sized>(
    n: usize,
    third_fields: &HashMap<&str, Vec<String>>,
    rng: &mut R,
) -> Percolator {
    let mut p = Percolator::default();
    (0..n)
        .map(|n| build_query(n, third_fields, rng))
        .for_each(|q| {
            p.add_query(Rc::new(q));
        });
    p
}

fn build_document<R: Rng + ?Sized>(
    _n: usize,
    third_fields: &HashMap<&str, Vec<String>>,
    rng: &mut R,
) -> Document {
    let d = Document::new()
        .with_value(FIELD, CountryName().fake_with_rng::<String, _>(rng))
        .with_value(FIELD2, one_random_data(&TASTE_VALUES, rng));
    let q3_field = one_random_data(&THIRD_FIELDS, rng);
    let q3_value = one_random_from_vec(third_fields.get(q3_field).unwrap(), rng);
    d.with_value(q3_field, q3_value)
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

    for nqueries in [10000, 50000, 100000] {
        group.throughput(Throughput::Elements(1));

        let mut rng = StdRng::seed_from_u64(42);

        // Build percolators with n queries field=valueN
        let mp = build_percolator::<StdRng>(nqueries, &third_fields, &mut rng);

        let input_size = criterion::BatchSize::SmallInput;

        group.bench_function(BenchmarkId::new("multi_perc", &mp), |b| {
            b.iter_batched(
                || build_document(nqueries, &third_fields, &mut rng),
                |d| black_box(mp.percolate(&d).next()),
                //|d| black_box(mp.hybrid_qids_from_document(&d).next()),
                //|d| black_box(mp.qids_from_document(&d).next()),
                //|d| black_box(mp.it_from_document(&d).next()),
                input_size,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, percolate_real);
criterion_main!(benches);
