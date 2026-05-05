#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SmartCache SmartCache;

typedef enum SmartCacheStatus {
    SMART_CACHE_STATUS_OK = 0,
    SMART_CACHE_STATUS_NULL_POINTER = 1,
    SMART_CACHE_STATUS_INVALID_UTF8 = 2,
    SMART_CACHE_STATUS_INVALID_POLICY = 3,
    SMART_CACHE_STATUS_NOT_FOUND = 4,
    SMART_CACHE_STATUS_INVALID_CONFIG = 5,
} SmartCacheStatus;

typedef enum SmartCachePolicy {
    SMART_CACHE_POLICY_LRU = 0,
    SMART_CACHE_POLICY_LFU = 1,
    SMART_CACHE_POLICY_FIFO = 2,
} SmartCachePolicy;

typedef struct SmartCacheStats {
    uint64_t hits;
    uint64_t misses;
    uint64_t puts;
    uint64_t updates;
    uint64_t removes;
    uint64_t evictions;
    uint64_t expirations;
} SmartCacheStats;

SmartCache* smart_cache_new(size_t capacity, int policy);
SmartCache* smart_cache_new_with_ttl(size_t capacity, int policy, uint64_t ttl_ms);
void smart_cache_free(SmartCache* cache);

SmartCacheStatus smart_cache_put(
    SmartCache* cache,
    const char* key,
    const uint8_t* value,
    size_t value_len
);

SmartCacheStatus smart_cache_get(
    SmartCache* cache,
    const char* key,
    uint8_t** out_ptr,
    size_t* out_len
);

SmartCacheStatus smart_cache_remove(SmartCache* cache, const char* key);
size_t smart_cache_len(const SmartCache* cache);
SmartCacheStatus smart_cache_stats(const SmartCache* cache, SmartCacheStats* out_stats);
void smart_cache_bytes_free(uint8_t* ptr, size_t len);

#ifdef __cplusplus
}
#endif
