use std::{collections::HashMap, rc::Rc};

use criterion::Throughput;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use mokapot::models::percolator::Percolator;
use mokapot::models::queries::ConjunctionQuery;
use mokapot::models::{documents::Document, queries::TermQuery};

const FIELD: &str = "field";
const FIELD2: &str = "second_field";

fn build_query(n: usize) -> ConjunctionQuery {
    let q1 = TermQuery::new(FIELD.into(), format!("value{n}").into());
    // Only 4 values for this one.
    let q2 = TermQuery::new(FIELD2.into(), format!("value{}", n % 4).into());
    ConjunctionQuery::new(vec![Box::new(q1), Box::new(q2)])
}

fn build_percolator(n: usize) -> Percolator {
    let mut p = Percolator::default();
    (0..n).map(build_query).for_each(|q| {
        p.add_query(Rc::new(q));
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
    let mut group = c.benchmark_group("Onefield_matching");

    for nqueries in [100, 10000, 50000] {
        group.throughput(Throughput::Elements(1));

        // Build percolators with n queries field=valueN
        let mp = build_percolator(nqueries);
        //let h = build_hashmap(nqueries);

        // Find the first decile value.
        let mid_value: Rc<str> = format!("value{}", nqueries / 10).into();
        let second_value: Rc<str> = format!("value{}", nqueries / 10 % 4).into();
        let d = Document::new()
            .with_value("field", mid_value.clone())
            .with_value("second_field", second_value);

        group.bench_with_input(BenchmarkId::new("multi_perc", &mp), &mp, |b, mp| {
            b.iter(|| mp.percolate(&d).next().unwrap())
        });

        /*group.bench_with_input(BenchmarkId::new("simple_perc", &p), &p, |b, p| {
            b.iter(|| p.qids_from_document(&d).next())
        });*/

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
