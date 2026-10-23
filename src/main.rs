use dkv::Cache;

fn main() {
    let mut cache = Cache::new();

    cache.set("name", "dkv");
    println!("Name: {:?}", cache.get("name"));
    println!("Number of entries: {}", cache.len());
    cache.delete("name");
    println!("Language exists: {}", cache.exists("name"));
}
