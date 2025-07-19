use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn add_two(x: i32) -> i32 {
    x + 2
}

fn benchmark_add_two(c: &mut Criterion) {
    c.bench_function("add_two", |b| b.iter(|| black_box(add_two(20))));
}

fn benchmark_native(c: &mut Criterion) {
    c.bench_function("native", |b| b.iter(|| black_box(2 + 20)));
}

criterion_group!(benches, benchmark_add_two, benchmark_native);
criterion_main!(benches);
