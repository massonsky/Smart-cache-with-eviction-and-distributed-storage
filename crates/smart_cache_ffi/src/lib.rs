use smart_cache_core::{Cache, CacheConfig, EvictionPolicy};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::slice;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartCacheStatus {
    Ok = 0,
    NullPointer = 1,
    InvalidUtf8 = 2,
    InvalidPolicy = 3,
    NotFound = 4,
    InvalidConfig = 5,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartCachePolicy {
    Lru = 0,
    Lfu = 1,
    Fifo = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SmartCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub puts: u64,
    pub updates: u64,
    pub removes: u64,
    pub evictions: u64,
    pub expirations: u64,
}

pub struct SmartCache {
    inner: Cache,
}

impl From<SmartCachePolicy> for EvictionPolicy {
    fn from(value: SmartCachePolicy) -> Self {
        match value {
            SmartCachePolicy::Lru => Self::Lru,
            SmartCachePolicy::Lfu => Self::Lfu,
            SmartCachePolicy::Fifo => Self::Fifo,
        }
    }
}

fn policy_from_i32(policy: c_int) -> Result<SmartCachePolicy, SmartCacheStatus> {
    match policy {
        0 => Ok(SmartCachePolicy::Lru),
        1 => Ok(SmartCachePolicy::Lfu),
        2 => Ok(SmartCachePolicy::Fifo),
        _ => Err(SmartCacheStatus::InvalidPolicy),
    }
}

unsafe fn parse_key<'a>(key: *const c_char) -> Result<&'a str, SmartCacheStatus> {
    if key.is_null() {
        return Err(SmartCacheStatus::NullPointer);
    }

    CStr::from_ptr(key)
        .to_str()
        .map_err(|_| SmartCacheStatus::InvalidUtf8)
}

#[no_mangle]
pub extern "C" fn smart_cache_new(capacity: usize, policy: c_int) -> *mut SmartCache {
    smart_cache_new_with_optional_ttl(capacity, policy, false, 0)
}

#[no_mangle]
pub extern "C" fn smart_cache_new_with_ttl(
    capacity: usize,
    policy: c_int,
    ttl_ms: u64,
) -> *mut SmartCache {
    smart_cache_new_with_optional_ttl(capacity, policy, true, ttl_ms)
}

fn smart_cache_new_with_optional_ttl(
    capacity: usize,
    policy: c_int,
    has_ttl: bool,
    ttl_ms: u64,
) -> *mut SmartCache {
    let Ok(policy) = policy_from_i32(policy) else {
        return ptr::null_mut();
    };

    let Ok(cache) = Cache::try_new(CacheConfig {
        capacity,
        policy: policy.into(),
        default_ttl_ms: has_ttl.then_some(ttl_ms),
    }) else {
        return ptr::null_mut();
    };

    Box::into_raw(Box::new(SmartCache { inner: cache }))
}

/// Frees a cache handle returned by `smart_cache_new` or `smart_cache_new_with_ttl`.
///
/// # Safety
///
/// `cache` must be either null or a pointer previously returned by this library. It must not be
/// freed more than once, and no other thread may use the handle while it is being freed.
#[no_mangle]
pub unsafe extern "C" fn smart_cache_free(cache: *mut SmartCache) {
    if !cache.is_null() {
        drop(Box::from_raw(cache));
    }
}

/// Inserts or replaces a value.
///
/// # Safety
///
/// `cache` must be a valid handle created by this library. `key` must point to a valid
/// null-terminated UTF-8 string. `value` must point to `value_len` readable bytes unless
/// `value_len` is zero.
#[no_mangle]
pub unsafe extern "C" fn smart_cache_put(
    cache: *mut SmartCache,
    key: *const c_char,
    value: *const u8,
    value_len: usize,
) -> SmartCacheStatus {
    if cache.is_null() || (value.is_null() && value_len != 0) {
        return SmartCacheStatus::NullPointer;
    }

    let key = match parse_key(key) {
        Ok(key) => key,
        Err(status) => return status,
    };

    let value = if value_len == 0 {
        Vec::new()
    } else {
        slice::from_raw_parts(value, value_len).to_vec()
    };

    (*cache).inner.put(key.to_owned(), value);
    SmartCacheStatus::Ok
}

/// Reads a value and returns a Rust-allocated byte buffer.
///
/// # Safety
///
/// `cache` must be a valid handle created by this library. `key` must point to a valid
/// null-terminated UTF-8 string. `out_ptr` and `out_len` must be valid writable pointers. On
/// success, the returned buffer must be released with `smart_cache_bytes_free`.
#[no_mangle]
pub unsafe extern "C" fn smart_cache_get(
    cache: *mut SmartCache,
    key: *const c_char,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> SmartCacheStatus {
    if cache.is_null() || out_ptr.is_null() || out_len.is_null() {
        return SmartCacheStatus::NullPointer;
    }

    *out_ptr = ptr::null_mut();
    *out_len = 0;

    let key = match parse_key(key) {
        Ok(key) => key,
        Err(status) => return status,
    };

    let Some(value) = (*cache).inner.get(key) else {
        return SmartCacheStatus::NotFound;
    };

    let mut boxed = value.into_boxed_slice();
    *out_len = boxed.len();
    *out_ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);

    SmartCacheStatus::Ok
}

/// Removes a key from the cache.
///
/// # Safety
///
/// `cache` must be a valid handle created by this library. `key` must point to a valid
/// null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn smart_cache_remove(
    cache: *mut SmartCache,
    key: *const c_char,
) -> SmartCacheStatus {
    if cache.is_null() {
        return SmartCacheStatus::NullPointer;
    }

    let key = match parse_key(key) {
        Ok(key) => key,
        Err(status) => return status,
    };

    if (*cache).inner.remove(key) {
        SmartCacheStatus::Ok
    } else {
        SmartCacheStatus::NotFound
    }
}

/// Returns the current number of live entries.
///
/// # Safety
///
/// `cache` must be null or a valid handle created by this library. No other thread may mutate the
/// same handle concurrently through the C ABI.
#[no_mangle]
pub unsafe extern "C" fn smart_cache_len(cache: *const SmartCache) -> usize {
    if cache.is_null() {
        return 0;
    }

    (*cache).inner.len()
}

/// Writes cache counters into `out_stats`.
///
/// # Safety
///
/// `cache` must be a valid handle created by this library. `out_stats` must be a valid writable
/// pointer. No other thread may mutate the same handle concurrently through the C ABI.
#[no_mangle]
pub unsafe extern "C" fn smart_cache_stats(
    cache: *const SmartCache,
    out_stats: *mut SmartCacheStats,
) -> SmartCacheStatus {
    if cache.is_null() || out_stats.is_null() {
        return SmartCacheStatus::NullPointer;
    }

    let stats = (*cache).inner.stats();
    *out_stats = SmartCacheStats {
        hits: stats.hits,
        misses: stats.misses,
        puts: stats.puts,
        updates: stats.updates,
        removes: stats.removes,
        evictions: stats.evictions,
        expirations: stats.expirations,
    };
    SmartCacheStatus::Ok
}

/// Frees a byte buffer returned by `smart_cache_get`.
///
/// # Safety
///
/// `ptr` and `len` must match a buffer returned by `smart_cache_get` from this library. The buffer
/// must be freed at most once.
#[no_mangle]
pub unsafe extern "C" fn smart_cache_bytes_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn c_abi_put_get_and_free_bytes() {
        unsafe {
            let cache = smart_cache_new(2, SmartCachePolicy::Lru as c_int);
            assert!(!cache.is_null());

            let key = CString::new("user:1").unwrap();
            let value = b"Alice";
            assert_eq!(
                smart_cache_put(cache, key.as_ptr(), value.as_ptr(), value.len()),
                SmartCacheStatus::Ok
            );

            let mut out_ptr = ptr::null_mut();
            let mut out_len = 0;
            assert_eq!(
                smart_cache_get(cache, key.as_ptr(), &mut out_ptr, &mut out_len),
                SmartCacheStatus::Ok
            );
            assert!(!out_ptr.is_null());
            assert_eq!(slice::from_raw_parts(out_ptr, out_len), value);

            smart_cache_bytes_free(out_ptr, out_len);
            smart_cache_free(cache);
        }
    }

    #[test]
    fn c_abi_reports_not_found() {
        unsafe {
            let cache = smart_cache_new(2, SmartCachePolicy::Lru as c_int);
            assert!(!cache.is_null());

            let key = CString::new("missing").unwrap();
            let mut out_ptr = ptr::null_mut();
            let mut out_len = 0;

            assert_eq!(
                smart_cache_get(cache, key.as_ptr(), &mut out_ptr, &mut out_len),
                SmartCacheStatus::NotFound
            );
            assert!(out_ptr.is_null());
            assert_eq!(out_len, 0);

            smart_cache_free(cache);
        }
    }
}
