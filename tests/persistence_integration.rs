use dkv::Cache;
use dkv::persistence::Persistence;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_log_path() -> std::path::PathBuf {
    let unique_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("dkv-test-{}.log", unique_id))
}

#[test]
fn recovers_set_operations() {
    let path = temporary_log_path();
    let persistence = Persistence::new(&path);

    persistence.append_set("name", "Alice").unwrap();
    persistence.append_set("city", "Berlin").unwrap();

    let mut cache = Cache::new();
    persistence.replay(&mut cache).unwrap();

    assert_eq!(cache.get("name"), Some("Alice"));
    assert_eq!(cache.get("city"), Some("Berlin"));

    fs::remove_file(path).unwrap();
}

#[test]
fn replays_delete_operations() {
    let path = temporary_log_path();
    let persistence = Persistence::new(&path);

    persistence.append_set("name", "Alice").unwrap();
    persistence.append_delete("name").unwrap();

    let mut cache = Cache::new();
    persistence.replay(&mut cache).unwrap();

    assert_eq!(cache.get("name"), None);

    fs::remove_file(path).unwrap();
}

#[test]
fn missing_log_file_produces_empty_cache() {
    let path = temporary_log_path();
    let persistence = Persistence::new(&path);

    let mut cache = Cache::new();
    persistence.replay(&mut cache).unwrap();

    assert!(cache.is_empty());
}
