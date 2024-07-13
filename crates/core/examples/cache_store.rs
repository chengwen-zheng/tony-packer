use toy_farm_core::{
    cache_store::{CacheStore, CacheStoreKey},
    Mode,
};

#[tokio::main]
async fn main() {
    let cache_store = CacheStore::new("./cache", "namespace", Mode::Development, "compiler");

    let store_key = CacheStoreKey {
        name: "example_module".to_string(),
        key: "hashed_key".to_string(),
    };

    let cache_data = vec![1, 2, 3, 4, 5];

    println!("{:?}", store_key.clone());
    // 写入缓存
    cache_store
        .write_single_cache(store_key.clone(), cache_data)
        .await
        .unwrap();

    // 读取缓存
    if let Some(data) = cache_store.read_cache("example_module") {
        println!("Cache data: {:?}", data);
    } else {
        println!("Cache not found");
    }
}
