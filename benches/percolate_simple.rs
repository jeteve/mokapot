use std::{collections::HashMap, rc::Rc};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use mokapot::models::{
    documents::Document,
    percolator::Percolator,
    queries::{Query, TermQuery},
};

fn build_simple_percolator(n: u32) -> Percolator {
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
struct JustAMap(HashMap<Rc<str>, usize>);
impl JustAMap {
    fn as_hashmap(&self) -> &HashMap<Rc<str>, usize> {
        &self.0
    }
}
impl std::fmt::Display for JustAMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "String HashMap with {} strings", self.0.len())
    }
}

fn build_hashmap(n: u32) -> JustAMap {
    let mut h = HashMap::new();
    for pretend_qid in 0..n {
        h.insert(format!("value{pretend_qid}").into(), pretend_qid as usize);
    }
    JustAMap(h)
}

fn percolate_simple(c: &mut Criterion) {
    // Build the percolators with 1000 simple queries.
    let p = build_simple_percolator(1000);
    let h = build_hashmap(1000);
    let value500: Rc<str> = "value500".into();
    let d = Document::new().with_value("field", value500.clone());

    let mut group = c.benchmark_group("Onefield_matching");

    group.bench_with_input(BenchmarkId::new("with_percolator", &p), &p, |b, p| {
        b.iter(|| p.qids_from_document(&d).next())
    });

    group.bench_with_input(BenchmarkId::new("with_hash", &h), &h, |b, h| {
        b.iter(|| h.as_hashmap().get(&value500.clone()))
    });

    group.finish();
}

criterion_group!(benches, percolate_simple);
criterion_main!(benches);

//criterion_group!(benches, benchmark_add_two, benchmark_native);
//criterion_main!(benches);
