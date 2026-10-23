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
