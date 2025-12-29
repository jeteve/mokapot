use std::collections::HashMap;
use std::hint::black_box;
use std::num::NonZeroUsize;

use criterion::Throughput;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use fake::Fake;
use fake::faker::address::en::CountryName;
use mokaccino::models::cnf::CNFQueryable;
use mokaccino::models::cnf::Query;

use mokaccino::models::document::Document;
use mokaccino::models::percolator_core::PercolatorCore;
use mokaccino::prelude::Percolator;
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
) -> Query {
    let q1 = FIELD.has_value(CountryName().fake_with_rng::<String, _>(rng));
    let q1b = FIELD2.has_value(one_random_data(&TASTE_VALUES, rng));

    let q3_field = one_random_data(&THIRD_FIELDS, rng);
    let q3_value = one_random_from_vec(third_fields.get(q3_field).unwrap(), rng);

    let q3 = if q3_field == "colour" {
        q3_field.has_prefix(
            q3_value
                .chars()
                .take(q3_value.len() - (1..(q3_value.len() - 1)).fake_with_rng::<usize, _>(rng))
                .collect::<String>(),
        )
    } else {
        q3_field.has_value(q3_value)
    };

    let q4 = "price".i64_lt((10000..100000).fake_with_rng::<i64, _>(rng));

    q1 & q1b & q3 & q4
}

fn build_percolator<R: Rng + ?Sized>(
    n: usize,
    third_fields: &HashMap<&str, Vec<String>>,
    rng: &mut R,
) -> Percolator {
    let mut p = Percolator::builder()
        .n_clause_matchers(NonZeroUsize::new(4).unwrap())
        .build();
    (0..n)
        .map(|n| build_query(n, third_fields, rng))
        .for_each(|q| {
            p.add_query(q);
        });

    println!("{}", p.stats());
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

    let price = (10000..100000).fake_with_rng::<i64, _>(rng).to_string();

    d.with_value(q3_field, q3_value).with_value("price", price)
}

#[cfg(not(tarpaulin_include))]
fn percolate_real(c: &mut Criterion) {
    let mut group = c.benchmark_group("Reallife_matching");

    let third_fields = HashMap::from([
        (
            "colour",
            vec![
                "blue".to_string(),
                "blue".to_string(),
                "bluegreen".to_string(),
                "bluegreenish".to_string(),
                "bluegreenishgrayish".to_string(),
                "bluegreenishgrayishsilverish".to_string(),
                "bluegreenishgrayishsilverishdottedpink".to_string(),
                "bluegreenishgrayishsilverishdottedpinkwithstripes".to_string(),
                "green".to_string(),
                "yellow".to_string(),
                "magenta".to_string(),
                "cyan".to_string(),
                "white".to_string(),
                "pink".to_string(),
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

        // Build percolators with n queries
        let mp = build_percolator(nqueries, &third_fields, &mut rng);

        let input_size = criterion::BatchSize::SmallInput;

        group.bench_function(BenchmarkId::new("multi_perc", &mp), |b| {
            b.iter_batched(
                || build_document(nqueries, &third_fields, &mut rng),
                |d| black_box(mp.percolate(&d).next()),
                input_size,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, percolate_real);
criterion_main!(benches);
