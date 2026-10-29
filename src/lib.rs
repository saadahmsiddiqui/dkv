pub mod server;
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct Entry {
    value: String,
    expires_at: Option<Instant>,
}

impl Entry {
    fn new(value: String) -> Self {
        Self {
            value,
            expires_at: None,
        }
    }

    fn with_ttl(value: String, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Some(Instant::now() + ttl),
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|deadline| Instant::now() >= deadline)
    }
}

pub struct Cache {
    entries: HashMap<String, Entry>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), Entry::new(value.into()));
    }

    pub fn set_with_ttl(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        ttl: Duration,
    ) {
        self.entries
            .insert(key.into(), Entry::with_ttl(value.into(), ttl));
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.value.as_str())
            }
        })
    }

    pub fn ttl(&self, key: &str) -> i64 {
        let Some(entry) = self.entries.get(key) else {
            return -2;
        };

        if entry.is_expired() {
            return -2;
        }

        match entry.expires_at {
            None => -1,
            Some(deadline) => {
                let seconds = deadline.saturating_duration_since(Instant::now()).as_secs();

                i64::try_from(seconds).unwrap_or(i64::MAX)
            }
        }
    }

    pub fn exists(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn delete(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    pub fn len(&self) -> usize {
        self.entries.values().filter(|e| !e.is_expired()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.entries.clear()
    }

    pub fn purge_expired(&mut self) {
        self.entries.retain(|_, e| !e.is_expired());
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_and_set_value() {
        let mut dkv_cache = Cache::new();
        dkv_cache.set("name".to_string(), "dkv".to_string());
        assert_eq!(dkv_cache.get("name"), Some("dkv"));
    }

    #[test]
    fn missing_key_returns_none() {
        let dkv_cache = Cache::new();
        assert_eq!(dkv_cache.get("None"), None);
    }

    #[test]
    fn set_replaces_existing_value() {
        let mut cache = Cache::new();
        cache.set("name".to_string(), "none".to_string());
        cache.set("name".to_string(), "dkv".to_string());
        assert_eq!(cache.get("name"), Some("dkv"));
    }

    #[test]
    fn exists_checks_key_presence() {
        let mut cache = Cache::new();
        assert!(!cache.exists("name"));
        cache.set("name".to_string(), "dkv".to_string());
        assert!(cache.exists("name"));
    }

    #[test]
    fn delete_removes_a_key() {
        let mut cache = Cache::new();
        cache.set("name".to_string(), "dkv".to_string());
        assert!(cache.exists("name"));
        cache.delete("name");
        assert!(!cache.exists("name"));
    }

    #[test]
    fn new_cache_is_empty() {
        let cache = Cache::default();
        assert!(cache.is_empty());
    }

    #[test]
    fn default_creates_an_empty_cache() {
        let cache = Cache::default();
        assert!(cache.is_empty());
    }

    #[test]
    fn len_tracks_unique_keys() {
        let mut cache = Cache::default();
        cache.set("1", "1");
        cache.set("2", "2");
        assert_eq!(cache.len(), 2);
        // Replacing does not increase key count
        cache.set("1", "2");
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn clear_removes_all_keys() {
        let mut cache = Cache::default();
        cache.set("1", "1");
        cache.set("2", "2");
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn sets_a_value_with_ttl() {
        let mut cache = Cache::default();
        cache.set_with_ttl("name", "dkv", Duration::from_secs(1));
        assert_eq!(cache.len(), 1);
    }
}
