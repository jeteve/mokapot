use std::{collections::HashMap, rc::Rc};

use criterion::Throughput;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use mokapot::models::percolator::MultiPercolator;
use mokapot::models::{
    documents::Document, percolator::Percolator, percolator::SimplePercolator, queries::TermQuery,
};

fn build_percolator<P>(n: u64) -> P
where
    P: Percolator + std::fmt::Display + Default,
{
    let mut p = P::default();
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

    for nqueries in [100, 1000, 2000, 5000, 10000] {
        group.throughput(Throughput::Elements(1));

        let p = build_percolator::<SimplePercolator>(nqueries);
        let mp = build_percolator::<MultiPercolator>(nqueries);
        let h = build_hashmap(nqueries);

        //group.bench_with_input(BenchmarkId::new("perc_dyna", &p), &p, |b, p| {
        //    b.iter(|| p.qids_from_document(&d).next())
        //});

        group.bench_with_input(BenchmarkId::new("multi_perc", &mp), &mp, |b, mp| {
            b.iter(|| mp.qids_from_document(&d).next())
        });

        /*group.bench_with_input(BenchmarkId::new("simple_perc", &p), &p, |b, p| {
            b.iter(|| p.qids_from_document(&d).next())
        });
        */

        /*
        group.bench_with_input(BenchmarkId::new("hashmap", &h), &h, |b, h| {
            b.iter(|| h.as_hashmap().get(&value500.clone()).map(|v| v.first()))
        });
        */
    }
    group.finish();
}

criterion_group!(benches, percolate_simple);
criterion_main!(benches);
