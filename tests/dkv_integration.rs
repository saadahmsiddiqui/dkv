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
}
