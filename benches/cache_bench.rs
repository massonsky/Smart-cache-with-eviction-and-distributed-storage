use smart_cache_core::{Cache, CacheConfig, EvictionPolicy};

fn main() {
    for policy in [
        EvictionPolicy::Lru,
        EvictionPolicy::Lfu,
        EvictionPolicy::Fifo,
    ] {
        let mut cache = Cache::new(CacheConfig {
            capacity: 10_000,
            policy,
            default_ttl_ms: None,
        });

        for i in 0..10_000 {
            cache.put(format!("key:{i}"), vec![42; 128]);
        }

        for i in 0..10_000 {
            let _ = cache.get(&format!("key:{i}"));
        }
    }
}
