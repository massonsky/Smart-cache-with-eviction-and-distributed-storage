#include <smart_cache.hpp>

#include <cassert>
#include <cstdint>
#include <vector>

int main() {
    smart_cache::SmartCache cache(2, smart_cache::EvictionPolicy::Lru);

    const std::vector<std::uint8_t> alice = {'A', 'l', 'i', 'c', 'e'};
    cache.put("user:1", alice);

    const auto value = cache.get("user:1");
    assert(value.has_value());
    assert(*value == alice);
    assert(cache.len() == 1);
}
