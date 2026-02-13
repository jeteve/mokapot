// Benchmark queries parsing and indexing.

use criterion::{Criterion, criterion_group, criterion_main};
use mokaccino::prelude::Query;
use std::hint::black_box;

#[cfg(not(tarpaulin_include))]
fn parsing_bench(c: &mut Criterion) {
    let mut rng = rand::rng();

    let mut group = c.benchmark_group("parsing-bench");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("parsing", |b| {
        b.iter_batched(
            || Query::random_string(&mut rng),
            |s| {
                black_box(s.parse::<mokaccino::prelude::Query>().unwrap());
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();

    // Test indexing queries in a percolator
    let mut group = c.benchmark_group("indexing-bench");
    group.throughput(criterion::Throughput::Elements(1));

    let mut p = mokaccino::prelude::Percolator::default();
    group.bench_function("indexing", |b| {
        b.iter_batched(
            || Query::random(&mut rng),
            |q| {
                black_box(p.add_query(q));
            },
            criterion::BatchSize::SmallInput,
        )
    });

    println!("{}", p.stats());

    group.finish();
}
criterion_group!(benches, parsing_bench);
criterion_main!(benches);
