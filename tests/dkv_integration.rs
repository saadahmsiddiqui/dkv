use dkv::Cache;

#[test]
fn dkv_supports_basic_operations() {
    let mut cache = Cache::new();
    cache.set("name", "dkv");
    assert_eq!(cache.get("name"), Some("dkv"));
    assert_eq!(cache.exists("name"), true);
    assert!(cache.delete("name"));
    assert_eq!(cache.exists("name"), false);
    assert_eq!(cache.get("name"), None);
}
