# About mokapot

mokapot is a percolator library in Rust. Condider this alpha software.

## About percolators

In search technology, a percolator is a component that allows the matching of a stream
of documents (for instance representing events) against a relatively static set
of queries (representing specific interests in passing events).

One very common use of a percolator is to implement instant alerting.

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

# Project URL

mokapot is developped at https://github.com/jeteve/mokapot/.
