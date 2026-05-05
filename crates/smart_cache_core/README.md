# smart_cache_core

Core Rust API for local smart cache usage.

## Features

- `EvictionPolicy::Lru`
- `EvictionPolicy::Lfu`
- `EvictionPolicy::Fifo`
- optional default TTL
- capacity-based eviction
- hit/miss/update/remove/eviction/expiration stats

## Example

```rust
use smart_cache_core::{Cache, CacheConfig, EvictionPolicy};

let mut cache = Cache::new(CacheConfig {
    capacity: 1000,
    policy: EvictionPolicy::Lru,
    default_ttl_ms: Some(60_000),
});

cache.put("user:1".to_owned(), b"Alice".to_vec());

assert_eq!(cache.get("user:1"), Some(b"Alice".to_vec()));
assert_eq!(cache.len(), 1);
```

## API

```rust
let mut cache = Cache::new(config);
cache.put(key, value);
cache.put_with_ttl_ms(key, value, Some(1000));
let value = cache.get("key");
let removed = cache.remove("key");
let len = cache.len();
let stats = cache.stats();
cache.clear();
```

## Tests

```bash
cargo test -p smart_cache_core
```

