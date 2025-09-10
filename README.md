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


# Getting started with development

This contains a dev container, and you should be able to just launch it using github codespace
or VSCode with devcontainers on your local machine.

The rest is plain Rust tooling:

```sh
# Should pass
cargo test
```

For now there is no other developer documentation, but reading the `tests`, specially the percolator ones
will be a good start.

# Benchmarking

```sh

cargo bench

# Or..
cargo bench --no-run

# And then run the benchmark executable by hand,
# for instance:
./target/release/deps/percolate_simple-(etc..) --bench

```

# Profiling a benchmark


## Using plain perf:

```sh

cargo bench --no-run
# Then run the executable with perf, for instance:

perf record target/release/deps/percolate_simple-0a1dd9fa04796dd8 --bench --profile-time 5

# Then do

perf report 

# etc..
```

## Using cargo flamegraph

See Also https://github.com/flamegraph-rs/flamegraph

```sh
# Generate the bench executable
cargo bench --no-run

# Then flamegraph it

cargo flamegraph --bench percolate_simple -- --bench --profile-time 5

# Inspect in text:

perf report --no-children
```


# Project URL

mokapot is developped at https://github.com/jeteve/mokapot/.


# Performance notes.

## On simple benchmark

Simple percolator =~ 5M Percolation/s
(vector based) Multi percolator: 50K Perc/s
(fixedbitset based) Multi percolator =~ 1M Perc/s 
