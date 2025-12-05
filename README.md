# About mokaccino

[![Rust](https://github.com/jeteve/mokapot/actions/workflows/rust.yml/badge.svg)](https://github.com/jeteve/mokapot/actions/workflows/rust.yml) [![Crates.io](https://img.shields.io/crates/v/mokaccino.svg)](https://crates.io/crates/mokaccino) [![codecov](https://codecov.io/github/jeteve/mokapot/graph/badge.svg?token=V822XHN6RK)](https://codecov.io/github/jeteve/mokapot)



mokaccino is a percolator library in Rust. Consider this beta software.

## About percolators

A Percolator is a component that allows the matching of a stream
of documents (for instance representing events) against a set
of queries (representing specific interests in events).

Another way of seeing it to be the dual of a search. In Search, you
match transient queries against a persistent set of documents and get the matching documents.
In Percolation, you match transient documents against a persistent set of queries and get the matching queries.

One very common use of a percolator is to implement instant alerting, where
incoming events are represented as Document and subscriptions represented as Queries.

Percolators usually exist as part of more general search products, like Lucene.


# Features

- Percolator first design.

- Performance focused.

- Supports any nested boolean queries, including negations.

- Prefix matching queries.

- Integer comparison queries.

- Geo queries.

- Query parsing

- `serde` serialisation/deserialisation (See Feature flags).

- Multithreaded environments support (See Feature flags)

- [Python binding](https://pypi.org/project/mokaccino/)

# Non-features

- Full-text search. For instance, this does not contain any document body tokenizing.


# Usage

In the first example test, we build a set of queries and check documents
will yield matching queries.


This supports query parsing for each query building user inputs via the `FromStr` trait.

You'll find some query syntax examples in the second example test. Use parenthesis to override classic
boolean operators' precedence.


# Example

```rust
use mokaccino::prelude::*;
use h3o::CellIndex;

fn test_percolator() {
    let mut p = Percolator::default();
    let q: Vec<Qid> = vec![
        p.add_query("A".has_value("a")),                         //0
        p.add_query("A".has_value("a") | "B".has_value("b")),    //1
        p.add_query("A".has_value("a") & "B".has_value("b")),    //2
        p.add_query(!"A".has_value("a")),                        //3
        p.add_query((!"A".has_value("a")) | "B".has_value("b")), //4
        p.add_query(!"A".has_value("a") & "B".has_value("b")),   //5
        p.add_query(!"A".has_value("a") & "A".has_value("a")),   //6 - should NEVER match anything.
        p.add_query("C".has_prefix("multi")),                    //7
        p.add_query("C".has_prefix("multi") & !"C".has_value("multimeter")), //8
        p.add_query(
            "A".has_value("aa") & "B".has_value("bb") & "C".has_value("cc") & "D".has_prefix("bla"),
        ), //9
        p.add_query("P".has_prefix("")),                         // 10
        p.add_query("L".i64_gt(1000)),                           // 11
        p.add_query("location".h3in("861f09b27ffffff".parse::<CellIndex>().unwrap())) // 12
    ];

    // See https://observablehq.com/@nrabinowitz/h3-index-inspector?collection=@nrabinowitz/h3
    assert_eq!(
        // The same location as the query one
        p.percolate(&[("location", "861f09b27ffffff")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4], q[12]]
    );

    assert_eq!(
        // This location is inside the query one.
        p.percolate(&[("location", "871f09b20ffffff")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4], q[12]]
    );

    assert_eq!(
        // This location is outside the query one.
        p.percolate(&[("location", "871f09b29ffffff")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&[("L", "1001")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4], q[11]]
    );

    assert_eq!(
        p.percolate(&[("P", "")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4], q[10]]
    );
    assert_eq!(
        p.percolate(&[("P", "some value")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[10]]
    );

    assert_eq!(
        p.percolate(&[("A", "aa"), ("B", "bb"), ("C", "cc"), ("D", "blabla")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[9]]
    );

    assert_eq!(
        p.percolate(&[("C", "mult")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );
    assert_eq!(
        p.percolate(&[("C", "multimeter")].into())
            .collect::<Vec<_>>(),
        vec![q[3], q[4], q[7]]
    );

    assert_eq!(
        p.percolate(&[("C", "multi")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4], q[7], q[8]]
    );

    assert_eq!(
        p.percolate(&[("X", "x")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&[("B", "b")].into()).collect::<Vec<_>>(),
        vec![q[1], q[3], q[4], q[5]]
    );

    assert_eq!(
        p.percolate(&[("A", "b")].into()).collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&[("A", "a")].into()).collect::<Vec<_>>(),
        vec![q[0], q[1]]
    );

    assert_eq!(
        p.percolate(&[("A", "a"), ("B", "b")].into())
            .collect::<Vec<_>>(),
        vec![q[0], q[1], q[2], q[4]]
    );

}

fn test_query_parsing(){
    // Query parsing test
    fn ps(s: &str) -> Query{
        s.parse().unwrap()
    }

    assert!("something".parse::<Query>().is_err());
    assert_eq!(ps("A:a"), "A".has_value("a"));
    assert_eq!(ps("A:123"), "A".has_value("123"));
    assert_eq!(ps("A:a OR B:b"), "A".has_value("a") | "B".has_value("b"));
    assert_eq!(ps("A:a AND B:b"), "A".has_value("a") & "B".has_value("b"));
    assert_eq!(ps("NOT A:a"), !"A".has_value("a"));
    assert_eq!(ps("NOT A:\"a a a\" OR B:b"), (!"A".has_value("a a a")) | "B".has_value("b"));
    assert_eq!(ps("NOT A:a AND B:b"), !"A".has_value("a") & "B".has_value("b"));
    assert_eq!(ps("NOT A:a AND A:a"), !"A".has_value("a") & "A".has_value("a"));
    assert_eq!(ps("C:multi*"), "C".has_prefix("multi"));
    assert_eq!(ps("C:\"mul \\\"ti\"* AND NOT C:multimeter"), "C".has_prefix("mul \"ti") & !"C".has_value("multimeter"));
    assert_eq!(ps("P:\"\"*"), "P".has_prefix(""));
    assert_eq!(ps("L<1000"), "L".i64_lt(1000));
    assert_eq!(ps("L<=1000"), "L".i64_le(1000));
    assert_eq!(ps("L=1000"), "L".i64_eq(1000));
    assert_eq!(ps("L>=1000"), "L".i64_ge(1000));
    assert_eq!(ps("L>1000"), "L".i64_gt(1000));
    assert_eq!(ps("location H3IN 861f09b27ffffff"), "location".h3in("861f09b27ffffff".parse::<CellIndex>().unwrap()))
}

test_percolator();
test_query_parsing();

```

# Feature flags

## serde

Use the feature flag `serde` if you want to Serialize/Deserialise the `Percolator` using Serde.

Usage in your Cargo.toml:

```toml
[dependencies]
mokaccino: { version = "0.2.0" , features = [ "serde" ] }
```

## send

Use the feature `send` if you want this crate to use only Send types.

# Application development guidelines

## Queries and Query IDs

Do not treat this crate's Query objects as your primary application objects.

Instead:

Turn your application objects (which can be query like or any other structure) into Queries,
index them using `add_query` and get `Qid`s.

Using `percolate` will give you an iterator on Qids, and its your application's business to match those
back to your original application objects.

## Documents

In the same spirit, do NOT use this crate's `Document` objects as your primary application objects.
Turn your incoming objects (which can be document like, or any other structure) into this crates's `Document`
and percolate to get `Qid`s.

## Serialisation

Using the `serde` feature, you can serialise the percolator for later deserialising.

The Query IDs  (`Qid`s)will of course stay the same accross serialising/deserialising cycles.

## Geographic Queries

mokaccino supports geographic queries through the H3 hexagonal hierarchical spatial index system. Geographic queries allow you to match documents based on their location within a specific H3 cell. This is particularly useful for location-based alerting and geofencing applications.

When querying, you specify an H3 CellIndex that defines a geographic region, and documents are matched if their location falls within that region or any of its child cells in the H3 hierarchy. This enables efficient spatial queries at any resolution level, from large regions down to very precise locations. For example, the query `"location".h3in("861f09b27ffffff".parse::<CellIndex>().unwrap())` will match any document whose location is within or contained by the specified H3 cell.

Alternatively, the query parser also support this via the syntax: `location H3IN 861f09b27ffffff`.

You can build any shape you like by building geo queries disjunctions, or using negations to make holes in your shape.

Reference: https://h3geo.org/

# Configuration optimisation

This comes with some printable statistics to help you decide on what parameters are best suited to
your use case.

To display statistics, simply do:

```rust
use mokaccino::prelude::*;
let p = Percolator::default();
// Add some queries and then
println!("{}", p.stats())
```

This will show you statistics that will help you tailor the parameters.
You can then use the percolator builder to tweak these parameters, like
in this example:

```rust
use mokaccino::prelude::*;
use std::num::NonZeroUsize;

let p = Percolator::builder()
         .n_clause_matchers(NonZeroUsize::new(3).unwrap())
         .prefix_sizes(vec![2, 6, 10, 50])
         .build();
```

## Clause matchers.

Method `n_clause_matchers` on the Percolator::builder()

The number of query clauses that are capable of being matched without further filtering.

Try to keep it within the 95% of your query clauses distribution shown by the stats (see above)

## Prefix sizes

Method `prefix_sizes` on the Percolator::builder()

This must be an ordered vector of usize that will help trimming down the number of
prefix related pre-heaters. Try to keep the number of pre-heaters low (see stats).
The stats also show a distribution of the prefix length in your queries to help you decide
on the best thresholds.

# Project URL

mokapot is developped at <https://github.com/jeteve/mokapot/>.


# Prior art

Luwak (Java), now part of Lucene, is a percolator deriving from
full text search principles.

https://github.com/flaxsearch/luwak?


Elastic Search Service percolator feature:

https://www.elastic.co/docs/reference/query-languages/query-dsl/query-dsl-percolate-query
