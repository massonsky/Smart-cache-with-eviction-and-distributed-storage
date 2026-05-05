#pragma once

#include "smart_cache_c.h"

#include <cstdint>
#include <optional>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace smart_cache {

enum class EvictionPolicy {
    Lru = SMART_CACHE_POLICY_LRU,
    Lfu = SMART_CACHE_POLICY_LFU,
    Fifo = SMART_CACHE_POLICY_FIFO,
};

class Error final : public std::runtime_error {
public:
    explicit Error(const std::string& message)
        : std::runtime_error(message) {}
};

class SmartCache final {
public:
    SmartCache(std::size_t capacity, EvictionPolicy policy)
        : handle_(smart_cache_new(capacity, static_cast<int>(policy))) {
        ensure_created();
    }

    SmartCache(std::size_t capacity, EvictionPolicy policy, std::uint64_t ttl_ms)
        : handle_(smart_cache_new_with_ttl(capacity, static_cast<int>(policy), ttl_ms)) {
        ensure_created();
    }

    ~SmartCache() {
        smart_cache_free(handle_);
    }

    SmartCache(const SmartCache&) = delete;
    SmartCache& operator=(const SmartCache&) = delete;

    SmartCache(SmartCache&& other) noexcept
        : handle_(std::exchange(other.handle_, nullptr)) {}

    SmartCache& operator=(SmartCache&& other) noexcept {
        if (this != &other) {
            smart_cache_free(handle_);
            handle_ = std::exchange(other.handle_, nullptr);
        }
        return *this;
    }

    void put(std::string_view key, std::span<const std::uint8_t> value) {
        const std::string stable_key(key);
        const auto status = smart_cache_put(
            handle_,
            stable_key.c_str(),
            value.data(),
            value.size()
        );
        throw_on_error(status, "smart_cache_put");
    }

    std::optional<std::vector<std::uint8_t>> get(std::string_view key) {
        const std::string stable_key(key);
        std::uint8_t* ptr = nullptr;
        std::size_t len = 0;

        const auto status = smart_cache_get(handle_, stable_key.c_str(), &ptr, &len);
        if (status == SMART_CACHE_STATUS_NOT_FOUND) {
            return std::nullopt;
        }
        throw_on_error(status, "smart_cache_get");

        std::vector<std::uint8_t> value(ptr, ptr + len);
        smart_cache_bytes_free(ptr, len);
        return value;
    }

    bool remove(std::string_view key) {
        const std::string stable_key(key);
        const auto status = smart_cache_remove(handle_, stable_key.c_str());
        if (status == SMART_CACHE_STATUS_NOT_FOUND) {
            return false;
        }
        throw_on_error(status, "smart_cache_remove");
        return true;
    }

    std::size_t len() const {
        return smart_cache_len(handle_);
    }

    SmartCacheStats stats() const {
        SmartCacheStats stats{};
        throw_on_error(smart_cache_stats(handle_, &stats), "smart_cache_stats");
        return stats;
    }

private:
    void ensure_created() const {
        if (handle_ == nullptr) {
            throw Error("failed to create SmartCache");
        }
    }

    static void throw_on_error(SmartCacheStatus status, const char* operation) {
        if (status == SMART_CACHE_STATUS_OK) {
            return;
        }
        throw Error(std::string(operation) + " failed with status " + std::to_string(status));
    }

    ::SmartCache* handle_ = nullptr;
};

} // namespace smart_cache
