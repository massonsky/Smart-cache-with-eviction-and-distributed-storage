# C++ Usage

The C++ API is a thin RAII wrapper over the Rust C ABI exported by `smart_cache_ffi`.

## Build Rust FFI Library

```bash
cargo build -p smart_cache_ffi
```

This creates the native library under `target/debug/`.

## Build C++ Example

```bash
cmake -S cpp -B target/cpp-build
cmake --build target/cpp-build
ctest --test-dir target/cpp-build --output-on-failure
```

## Example

```cpp
#include <smart_cache.hpp>

#include <cassert>
#include <cstdint>
#include <vector>

int main() {
    smart_cache::SmartCache cache(1000, smart_cache::EvictionPolicy::Lru);

    const std::vector<std::uint8_t> alice = {'A', 'l', 'i', 'c', 'e'};
    cache.put("user:1", alice);

    const auto value = cache.get("user:1");
    assert(value.has_value());
    assert(*value == alice);
}
```

## Ownership Rule

All memory allocated by Rust is freed by Rust. C++ callers must not free buffers returned by `smart_cache_get` manually; the wrapper calls `smart_cache_bytes_free` after copying data into `std::vector<std::uint8_t>`.

