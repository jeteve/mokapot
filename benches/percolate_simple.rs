use std::{collections::HashMap, rc::Rc};

use criterion::Throughput;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use mokapot::models::{documents::Document, percolator::Percolator, queries::TermQuery};

fn build_simple_percolator(n: u64) -> Percolator {
    let mut p = Percolator::new();
    let field: Rc<str> = "field".into();
    (0..n)
        .map(|n| Rc::new(TermQuery::new(field.clone(), format!("value{n}").into())))
        .for_each(|q| {
            p.add_query(q);
        });
    p
}

// Compare with simple hashmap access
struct JustAMap(HashMap<Rc<str>, Vec<usize>>);
impl JustAMap {
    fn as_hashmap(&self) -> &HashMap<Rc<str>, Vec<usize>> {
        &self.0
    }
}
impl std::fmt::Display for JustAMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HashMap-{} strings", self.0.len())
    }
}

fn build_hashmap(n: u64) -> JustAMap {
    let mut h: HashMap<Rc<str>, Vec<usize>> = HashMap::new();
    for pretend_qid in 0..n {
        h.insert(
            format!("value{pretend_qid}").into(),
            vec![pretend_qid.try_into().unwrap()],
        );
    }
    JustAMap(h)
}

fn percolate_simple(c: &mut Criterion) {
    // Build the percolators with 1000 simple queries.
    let value500: Rc<str> = "value500".into();
    let d = Document::new().with_value("field", value500.clone());

    let mut group = c.benchmark_group("Onefield_matching");

    for nqueries in [1, 10, 100, 1000, 10000, 100000, 1000000] {
        group.throughput(Throughput::Elements(nqueries));

        let p = build_simple_percolator(nqueries);
        let h = build_hashmap(nqueries);

        group.bench_with_input(BenchmarkId::new("perc_dyna", &p), &p, |b, p| {
            b.iter(|| p.qids_from_document(&d).next())
        });

        group.bench_with_input(BenchmarkId::new("perc_static", &p), &p, |b, p| {
            b.iter(|| p.static_qids_from_document(&d).next())
        });

        group.bench_with_input(BenchmarkId::new("hashmap", &h), &h, |b, h| {
            b.iter(|| h.as_hashmap().get(&value500.clone()).map(|v| v.first()))
        });
    }
    group.finish();
}

criterion_group!(benches, percolate_simple);
criterion_main!(benches);

//criterion_group!(benches, benchmark_add_two, benchmark_native);
//criterion_main!(benches);
