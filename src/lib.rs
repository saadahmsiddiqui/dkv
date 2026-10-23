pub mod server;
use std::collections::HashMap;

pub struct Cache {
    store: HashMap<String, String>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.store.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.store.get(key).map(|value| value.as_str())
    }

    pub fn exists(&self, key: &str) -> bool {
        self.store.contains_key(key)
    }

    pub fn delete(&mut self, key: &str) -> bool {
        self.store.remove(key).is_some()
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn clear(&mut self) {
        self.store.clear()
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
}
