# Smart Cache RS

Rust-first smart cache with eviction policies, TTL, C/C++ ABI, Python bindings, and an initial distributed storage layer.

## Components

- `crates/smart_cache_core`: local cache core, Rust API, eviction, TTL, stats.
- `crates/smart_cache_ffi`: stable C ABI for C and C++ integration.
- `crates/smart_cache_py`: Python native module through PyO3 and maturin.
- `crates/smart_cache_dist`: distributed routing and HTTP API foundation.
- `cpp`: C/C++ headers and C++ RAII wrapper.
- `python`: Python usage notes and tests.
- `benches`: benchmark entry points.

## Build and Test

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Rust

See [crates/smart_cache_core/README.md](crates/smart_cache_core/README.md).

## Python

See [python/README.md](python/README.md).

## C++

See [cpp/README.md](cpp/README.md).

## Current Status

The local Rust core, FFI boundary, Python binding, and C++ smoke example are implemented and covered by basic tests. The distributed crate currently provides cluster/routing primitives and an HTTP app skeleton; replication and quorum behavior are next-stage work.

