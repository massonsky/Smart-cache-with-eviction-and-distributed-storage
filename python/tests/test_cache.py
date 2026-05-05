import time

from smart_cache import Cache, EvictionPolicy


def test_put_get_bytes():
    cache = Cache(capacity=1000, policy=EvictionPolicy.LRU)

    cache.put("user:1", b"Alice")

    assert cache.get("user:1") == b"Alice"
    assert len(cache) == 1


def test_lru_eviction():
    cache = Cache(capacity=2, policy=EvictionPolicy.LRU)
    cache.put("a", b"1")
    cache.put("b", b"2")
    assert cache.get("a") == b"1"

    cache.put("c", b"3")

    assert cache.get("a") == b"1"
    assert cache.get("b") is None
    assert cache.get("c") == b"3"


def test_ttl():
    cache = Cache(capacity=2, policy=EvictionPolicy.LRU, ttl_ms=10)
    cache.put("a", b"1")
    time.sleep(0.025)

    assert cache.get("a") is None


def test_stats():
    cache = Cache(capacity=2, policy=EvictionPolicy.LRU)
    cache.put("a", b"1")
    assert cache.get("a") == b"1"
    assert cache.get("missing") is None

    stats = cache.stats()
    assert stats.hits == 1
    assert stats.misses == 1
    assert stats.puts == 1
