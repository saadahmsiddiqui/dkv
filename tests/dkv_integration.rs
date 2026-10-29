use dkv::Cache;

#[test]
fn dkv_supports_basic_operations() {
    let mut cache = Cache::new();
    cache.set("name", "dkv");
    assert_eq!(cache.get("name"), Some("dkv"));
    assert!(cache.exists("name"));
    assert!(cache.delete("name"));
    assert!(!cache.exists("name"));
    assert_eq!(cache.get("name"), None);
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
    cache.set("name", "dkv");
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn ttl_entry_is_available_before_expiration() {
    use std::time::Duration;

    let mut cache = Cache::new();
    cache.set_with_ttl("session", "Alice", Duration::from_secs(10));

    assert_eq!(cache.get("session"), Some("Alice"));
}

#[test]
fn ttl_entry_expires() {
    use std::thread;
    use std::time::Duration;

    let mut cache = Cache::new();
    cache.set_with_ttl("session", "Alice", Duration::from_millis(30));

    assert_eq!(cache.get("session"), Some("Alice"));

    thread::sleep(Duration::from_millis(60));

    assert_eq!(cache.get("session"), None);
    assert!(!cache.exists("session"));
}

#[test]
fn set_replaces_ttl_with_permanent_entry() {
    use std::thread;
    use std::time::Duration;

    let mut cache = Cache::new();
    cache.set_with_ttl("key", "temporary", Duration::from_millis(30));
    cache.set("key", "permanent");

    thread::sleep(Duration::from_millis(60));

    assert_eq!(cache.get("key"), Some("permanent"));
}

#[test]
fn purge_expired_removes_entries() {
    use std::thread;
    use std::time::Duration;

    let mut cache = Cache::new();
    cache.set_with_ttl("short", "value", Duration::from_millis(30));
    cache.set("permanent", "value");

    thread::sleep(Duration::from_millis(60));
    cache.purge_expired();

    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get("permanent"), Some("value"));
    assert!(!cache.exists("short"));
}
