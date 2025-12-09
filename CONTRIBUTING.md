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

# Performance notes.

## On simple benchmark

Simple percolator =~ 5M Percolation/s
(vector based) Multi percolator: 50K Perc/s
(fixedbitset based) Multi percolator =~ 1M Perc/s 

# Mutation testing

use https://mutants.rs/
