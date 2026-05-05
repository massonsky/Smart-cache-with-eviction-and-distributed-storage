use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    Fifo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheConfig {
    pub capacity: usize,
    pub policy: EvictionPolicy,
    pub default_ttl_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub puts: u64,
    pub updates: u64,
    pub removes: u64,
    pub evictions: u64,
    pub expirations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    ZeroCapacity,
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => f.write_str("cache capacity must be greater than zero"),
        }
    }
}

impl Error for CacheError {}

#[derive(Debug, Clone)]
struct Entry {
    value: Vec<u8>,
    inserted_seq: u64,
    last_access_seq: u64,
    frequency: u64,
    expires_at: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FifoRecord {
    inserted_seq: u64,
    key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LruRecord {
    access_seq: u64,
    key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TtlRecord {
    expires_at: Instant,
    key: String,
}

pub struct Cache {
    config: CacheConfig,
    entries: HashMap<String, Entry>,
    fifo: VecDeque<FifoRecord>,
    lru: VecDeque<LruRecord>,
    ttl_heap: BinaryHeap<Reverse<TtlRecord>>,
    seq: u64,
    stats: CacheStats,
}

impl Cache {
    pub fn new(config: CacheConfig) -> Self {
        Self::try_new(config).expect("invalid cache config")
    }

    pub fn try_new(config: CacheConfig) -> Result<Self, CacheError> {
        if config.capacity == 0 {
            return Err(CacheError::ZeroCapacity);
        }

        Ok(Self {
            config,
            entries: HashMap::new(),
            fifo: VecDeque::new(),
            lru: VecDeque::new(),
            ttl_heap: BinaryHeap::new(),
            seq: 0,
            stats: CacheStats::default(),
        })
    }

    pub fn put(&mut self, key: String, value: Vec<u8>) {
        self.put_with_ttl_ms(key, value, self.config.default_ttl_ms);
    }

    pub fn put_with_ttl_ms(&mut self, key: String, value: Vec<u8>, ttl_ms: Option<u64>) {
        self.purge_expired();
        let seq = self.next_seq();
        self.stats.puts = self.stats.puts.saturating_add(1);

        let expires_at = ttl_ms.map(|ms| Instant::now() + Duration::from_millis(ms));

        if let Some(entry) = self.entries.get_mut(&key) {
            entry.value = value;
            entry.last_access_seq = seq;
            entry.frequency = entry.frequency.saturating_add(1);
            entry.expires_at = expires_at;
            self.stats.updates = self.stats.updates.saturating_add(1);
            self.lru.push_back(LruRecord {
                access_seq: seq,
                key: key.clone(),
            });
            self.push_ttl_record(&key, expires_at);
            return;
        }

        self.entries.insert(
            key.clone(),
            Entry {
                value,
                inserted_seq: seq,
                last_access_seq: seq,
                frequency: 1,
                expires_at,
            },
        );
        self.fifo.push_back(FifoRecord {
            inserted_seq: seq,
            key: key.clone(),
        });
        self.lru.push_back(LruRecord {
            access_seq: seq,
            key: key.clone(),
        });
        self.push_ttl_record(&key, expires_at);

        while self.entries.len() > self.config.capacity {
            if !self.evict_one() {
                break;
            }
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.purge_expired();
        let seq = self.next_seq();

        let Some(entry) = self.entries.get_mut(key) else {
            self.stats.misses = self.stats.misses.saturating_add(1);
            return None;
        };

        if Self::entry_is_expired(entry) {
            self.entries.remove(key);
            self.stats.expirations = self.stats.expirations.saturating_add(1);
            self.stats.misses = self.stats.misses.saturating_add(1);
            return None;
        }

        entry.last_access_seq = seq;
        entry.frequency = entry.frequency.saturating_add(1);
        self.lru.push_back(LruRecord {
            access_seq: seq,
            key: key.to_owned(),
        });
        self.stats.hits = self.stats.hits.saturating_add(1);
        Some(entry.value.clone())
    }

    pub fn contains_key(&mut self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed {
            self.stats.removes = self.stats.removes.saturating_add(1);
        }
        removed
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.fifo.clear();
        self.lru.clear();
        self.ttl_heap.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.config.capacity
    }

    pub fn policy(&self) -> EvictionPolicy {
        self.config.policy
    }

    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    fn next_seq(&mut self) -> u64 {
        self.seq = self.seq.saturating_add(1);
        self.seq
    }

    fn push_ttl_record(&mut self, key: &str, expires_at: Option<Instant>) {
        if let Some(expires_at) = expires_at {
            self.ttl_heap.push(Reverse(TtlRecord {
                expires_at,
                key: key.to_owned(),
            }));
        }
    }

    fn purge_expired(&mut self) {
        let now = Instant::now();

        while let Some(Reverse(record)) = self.ttl_heap.peek() {
            if record.expires_at > now {
                break;
            }

            let Reverse(record) = self.ttl_heap.pop().expect("peeked ttl record must exist");
            let is_current_expiration = self
                .entries
                .get(&record.key)
                .is_some_and(|entry| entry.expires_at == Some(record.expires_at));

            if is_current_expiration {
                self.entries.remove(&record.key);
                self.stats.expirations = self.stats.expirations.saturating_add(1);
            }
        }
    }

    fn evict_one(&mut self) -> bool {
        match self.config.policy {
            EvictionPolicy::Lru => self.evict_lru(),
            EvictionPolicy::Lfu => self.evict_lfu(),
            EvictionPolicy::Fifo => self.evict_fifo(),
        }
    }

    fn evict_lru(&mut self) -> bool {
        while let Some(record) = self.lru.pop_front() {
            let Some(entry) = self.entries.get(&record.key) else {
                continue;
            };

            if entry.last_access_seq == record.access_seq {
                self.entries.remove(&record.key);
                self.stats.evictions = self.stats.evictions.saturating_add(1);
                return true;
            }
        }
        false
    }

    fn evict_fifo(&mut self) -> bool {
        while let Some(record) = self.fifo.pop_front() {
            let Some(entry) = self.entries.get(&record.key) else {
                continue;
            };

            if entry.inserted_seq == record.inserted_seq {
                self.entries.remove(&record.key);
                self.stats.evictions = self.stats.evictions.saturating_add(1);
                return true;
            }
        }
        false
    }

    fn evict_lfu(&mut self) -> bool {
        let Some(key) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| (entry.frequency, entry.last_access_seq, entry.inserted_seq))
            .map(|(key, _)| key.clone())
        else {
            return false;
        };

        self.entries.remove(&key);
        self.stats.evictions = self.stats.evictions.saturating_add(1);
        true
    }

    fn entry_is_expired(entry: &Entry) -> bool {
        entry
            .expires_at
            .is_some_and(|expires_at| expires_at <= Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn cache(policy: EvictionPolicy) -> Cache {
        Cache::new(CacheConfig {
            capacity: 2,
            policy,
            default_ttl_ms: None,
        })
    }

    #[test]
    fn rejects_zero_capacity() {
        let result = Cache::try_new(CacheConfig {
            capacity: 0,
            policy: EvictionPolicy::Lru,
            default_ttl_ms: None,
        });

        assert_eq!(result.err(), Some(CacheError::ZeroCapacity));
    }

    #[test]
    fn lru_evicts_least_recently_used_key() {
        let mut cache = cache(EvictionPolicy::Lru);
        cache.put("a".into(), b"1".to_vec());
        cache.put("b".into(), b"2".to_vec());
        assert_eq!(cache.get("a"), Some(b"1".to_vec()));

        cache.put("c".into(), b"3".to_vec());

        assert_eq!(cache.get("a"), Some(b"1".to_vec()));
        assert_eq!(cache.get("b"), None);
        assert_eq!(cache.get("c"), Some(b"3".to_vec()));
    }

    #[test]
    fn fifo_evicts_oldest_inserted_key() {
        let mut cache = cache(EvictionPolicy::Fifo);
        cache.put("a".into(), b"1".to_vec());
        cache.put("b".into(), b"2".to_vec());
        assert_eq!(cache.get("a"), Some(b"1".to_vec()));

        cache.put("c".into(), b"3".to_vec());

        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b"), Some(b"2".to_vec()));
        assert_eq!(cache.get("c"), Some(b"3".to_vec()));
    }

    #[test]
    fn lfu_evicts_lowest_frequency_key() {
        let mut cache = cache(EvictionPolicy::Lfu);
        cache.put("a".into(), b"1".to_vec());
        cache.put("b".into(), b"2".to_vec());
        assert_eq!(cache.get("a"), Some(b"1".to_vec()));

        cache.put("c".into(), b"3".to_vec());

        assert_eq!(cache.get("a"), Some(b"1".to_vec()));
        assert_eq!(cache.get("b"), None);
        assert_eq!(cache.get("c"), Some(b"3".to_vec()));
    }

    #[test]
    fn ttl_expires_key() {
        let mut cache = Cache::new(CacheConfig {
            capacity: 2,
            policy: EvictionPolicy::Lru,
            default_ttl_ms: Some(10),
        });

        cache.put("a".into(), b"1".to_vec());
        thread::sleep(Duration::from_millis(25));

        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.stats().expirations, 1);
    }

    #[test]
    fn updating_key_refreshes_ttl() {
        let mut cache = Cache::new(CacheConfig {
            capacity: 2,
            policy: EvictionPolicy::Lru,
            default_ttl_ms: Some(30),
        });

        cache.put("a".into(), b"1".to_vec());
        thread::sleep(Duration::from_millis(20));
        cache.put("a".into(), b"2".to_vec());
        thread::sleep(Duration::from_millis(20));

        assert_eq!(cache.get("a"), Some(b"2".to_vec()));
    }

    #[test]
    fn capacity_overflow_keeps_len_under_capacity() {
        let mut cache = cache(EvictionPolicy::Fifo);
        cache.put("a".into(), b"1".to_vec());
        cache.put("b".into(), b"2".to_vec());
        cache.put("c".into(), b"3".to_vec());

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn remove_returns_false_for_missing_key() {
        let mut cache = cache(EvictionPolicy::Lru);

        assert!(!cache.remove("missing"));
        cache.put("a".into(), b"1".to_vec());
        assert!(cache.remove("a"));
        assert_eq!(cache.get("a"), None);
    }

    #[test]
    fn stats_counts_hits_and_misses() {
        let mut cache = cache(EvictionPolicy::Lru);
        cache.put("a".into(), b"1".to_vec());

        assert_eq!(cache.get("a"), Some(b"1".to_vec()));
        assert_eq!(cache.get("b"), None);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.puts, 1);
    }
}
