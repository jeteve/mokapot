# About mokapot

mokapot is a percolator library in Rust. Consider this beta software.

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

# Non-features

- Full text search. For instance this does not contain any document body tokenizing.

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
        p.add_query("C".has_prefix("multi")),                    //7
        p.add_query("C".has_prefix("multi") & !"C".has_value("multimeter")), //8
        p.add_query(
            "A".has_value("aa") & "B".has_value("bb") & "C".has_value("cc") & "D".has_prefix("bla"),
        ), //9
    ];

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

# Project URL

mokapot is developped at <https://github.com/jeteve/mokapot/>.

