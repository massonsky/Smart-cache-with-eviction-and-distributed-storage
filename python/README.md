# Python Usage

The Python package is built from `crates/smart_cache_py` with PyO3 and maturin.

## Build Wheel

From the repository root:

```bash
python -m venv target/py-venv
target/py-venv/bin/python -m pip install maturin pytest
target/py-venv/bin/maturin build --release -o target/wheels
```

The wheel is written to `target/wheels/`.

## Install Locally

```bash
target/py-venv/bin/python -m pip install target/wheels/smart_cache-*.whl
```

For editable development:

```bash
VIRTUAL_ENV=target/py-venv target/py-venv/bin/maturin develop
target/py-venv/bin/pytest python/tests
```

## Example

```python
from smart_cache import Cache, EvictionPolicy

cache = Cache(capacity=1000, policy=EvictionPolicy.LRU, ttl_ms=60000)
cache.put("user:1", b"Alice")

assert cache.get("user:1") == b"Alice"
assert len(cache) == 1

stats = cache.stats()
print(stats.hits, stats.misses)
```

## Tests

```bash
target/py-venv/bin/pytest python/tests
```

