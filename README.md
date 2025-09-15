# About mokapot

mokapot is a percolator library in Rust. Consider this alpha software.

## About percolators

In search technology, a percolator is a component that allows the matching of a stream
of documents (for instance representing events) against a relatively static set
of queries (representing specific interests in events).

One very common use of a percolator is to implement instant alerting, when you consider incoming
document as events.

# Features

- Percolator first design

- Performance and correctness focused.

- Support for any boolean queries.

- Support for prefix matching.

- Support for number comparison queries (coming up)

# Non-features

- Full text search. This does not contain any document body tokenizing.

# Example

```rust
use mokapot::prelude::*;

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
    ];

    assert_eq!(
        p.percolate(&Document::default().with_value("X", "x"))
            .collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&Document::default().with_value("B", "b"))
            .collect::<Vec<_>>(),
        vec![q[1], q[3], q[4], q[5]]
    );

    assert_eq!(
        p.percolate(&Document::default().with_value("A", "b"))
            .collect::<Vec<_>>(),
        vec![q[3], q[4]]
    );

    assert_eq!(
        p.percolate(&Document::default().with_value("A", "a"))
            .collect::<Vec<_>>(),
        vec![q[0], q[1]]
    );

    assert_eq!(
        p.percolate(
            &Document::default()
                .with_value("A", "a")
                .with_value("B", "b")
        )
        .collect::<Vec<_>>(),
        vec![q[0], q[1], q[2], q[4]]
    );
}

```

# Project URL

mokapot is developped at https://github.com/jeteve/mokapot/.

