# Changelog

All notable changes to this project will be documented in this file.

## [0.8.0]
* Added `optimized` and `compacted` methods to `Percolator` for automatic optimization and query compaction respectively.
* Added `holes_ratio` method to `Percolator` to calculate the ratio of removed to added queries.
* Enhanced the percolator's stats object with properties `recommended_cmcount` and `recommended_prefix_sizes` based on usage statistics.
* Added `n_queries_removed` to the stats object to keep track of removed query stats.
* Optimized preheater logic by using `expand_clause` directly.
* Added `with_config` method to the percolator builder.

## [0.7.0]
* Added explicit `PercolatorUid` to map custom query IDs to queries, allowing the use of custom query types.
* Renamed `Percolator` to `PercolatorCore`, replacing it with a new `Percolator` wrapper around `PercolatorCore` with automated query IDs.
* Added `remove_qid` to allow unindexing queries by query ID.
* Added `unindex_docid` functionality to allow unindexing queries.
* Added `safe_add_query` and `safe_add_query_with_uid` that return `PercolatorError` on failure (e.g. prefix too long, too many queries/clauses/preheaters).
* Implemented `with_value_mut` on `Document` to mutate a document in-place, which is more memory-efficient.
* Implemented literal latlng within querying (`LATLNG_WITHIN`) with support for parsing `lat,lng,radius` strings.
* Added support for pure lat/lng queries matching using `LatLngWithinQuery` with H3 disk covering optimizations.
* Improved protection against empty disk covering for zero radii in geospatial tools.
* Moved the `percolator` module to `percolator_core`.

## [0.6.0]
* Added H3 geospatial inside query (`H3InsideQuery`) and parsing (`h3in`).
* Integrated H3 geospatial inside query into percolation.
* Implemented benchmarking for parsing and indexing queries.
* Moved parsing implementation to `parsing` module and enabled generation of random syntactically correct queries.
* Renamed AST types for clarity.
* Optimized preheaters logic to avoid cloning.
* Sorted match items by cost for better performance.
* Avoided double escaping in formatting.

## [0.5.0]
* Added parser for string representation of queries.
* Implemented recursive parsing, enabling parsing of flat queries and unary logic.
* Added parsing capabilities for text field values.

## [0.4.1]
* Enabled `send` feature touching `ExpanderF`.
* Added a separate Rust CI job to run code coverage.

## [0.4.0]
* Added the `send` feature for sending types across threads.
* Abstracted `Rc` based types for generic reference counting.
* Reached 100% test coverage for the percolator.

## [0.3.0]
* Implemented numeric query matching with support for interger comparisons (`i64_gt`, `i64_lt`, `i64_eq`, etc.).
* Implemented ordered query and proper indexing of integer comparison queries.
* Implemented `fibo_floor` and `fibo_ceil` logic.
* Optimized preheating for queries.

## [0.2.0]
* Added configuration options through a separate `Config` struct (`PercolatorConfig`).
* Allowed configuration of prefix sizes via the `PercBuilder`.
* Added statistics reporting (`PercolatorStats`).
* Extracted percolator tools and preheaters to a separate module.
* Optimized prefix taking and enabled clipping prefixes for fewer preheaters.

## [0.1.0]
