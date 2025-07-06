use criterion::{black_box, criterion_group, criterion_main, Criterion};

use mokapot::add_two;

fn benchmark_add_two(c: &mut Criterion) {
    c.bench_function("add_two", |b| b.iter(|| add_two(black_box(20))));
}

fn benchmark_native(c: &mut Criterion) {
    c.bench_function("native", |b| b.iter(|| 2 + black_box(20)));
}

criterion_group!(benches, benchmark_add_two, benchmark_native);
criterion_main!(benches);
