# About mokapot

mokapot is a percolator library in Rust. Condider this alpha software.

## About percolators

In search technology, a percolator is a component that allows the matching of a stream
of documents (for instance representing events) against a relatively static set
of queries (representing specific interests in events).

One very common use of a percolator is to implement instant alerting, when you consider incoming
document as events.

# Features

- Percolator first design

- Support for fuzzy queries (coming up)

- Support for number comparison queries (coming up)

# Non-features

- Full text search. This does not contain any document body tokenizing.

# Example

```rust

use mokapot::prelude::*;
fn test_percolator() {
    let mut p = Percolator::default();
    let q: Vec<Qid> = vec![
        p.add_query("A".has_value("a")),
        p.add_query("A".has_value("a") | "B".has_value("b")),
        p.add_query("A".has_value("a") & "B".has_value("b")),
    ];

    assert_eq!(
        p.percolate(&Document::default().with_value("X", "x"))
            .collect_vec(),
        vec![]
    );

    assert_eq!(
        p.percolate(&Document::default().with_value("A", "a"))
            .collect_vec(),
        vec![q[0], q[1]]
    );

    assert_eq!(
        p.percolate(
            &Document::default()
                .with_value("A", "a")
                .with_value("B", "b")
        )
        .collect_vec(),
        vec![q[0], q[1], q[2]]
    );
}

```

# Project URL

mokapot is developped at https://github.com/jeteve/mokapot/.

