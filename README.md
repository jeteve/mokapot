# About mokaccino

mokaccino is a percolator library in Rust. Consider this beta software.

## About percolators

In search technology, a percolator is a component that allows the matching of a stream
of documents (for instance representing events) against a relatively static set
of queries (representing specific interests in events).

One very common use of a percolator is to implement instant alerting, when you consider incoming
document as events.

# Features

- Percolator first design.

- Performance focused.

- Support for any nested boolean queries, including negations.

- Support for prefix matching.

- Support for `serde` serialisation/deserialisation. (See Feature flags)

# Non-features

- Full text search. For instance this does not contain any document body tokenizing.

# Example

```rust
use mokaccino::prelude::*;

#[test]
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
    ];

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
```

# Feature flags

Use the feature flag `serde` if you want to Serialize/Deserialise the `Percolator` using Serde.

Usage in your Cargo.toml:

```toml
[dependencies]
mokaccino: { version = "0.2.0" , features = [ "serde" ] }
```

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

