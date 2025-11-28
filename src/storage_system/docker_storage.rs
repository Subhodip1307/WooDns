use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug,Clone)]
pub struct DockerStorage {
    container: Arc<RwLock<HashMap<String, String>>>,
}
impl DockerStorage {
    pub fn new() -> Self {
        // createing new instance of the storage
        Self {
            container: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn get(&self, query: &String) -> Option<String> {
        let data_reader = self.container.read().await;
        data_reader.get(query).cloned()
    }
    pub async fn set(&self, query: String, value: String) {
        let mut data_writer = self.container.write().await;
        data_writer.insert(query, value);
    }
    pub async fn contains(&self, query: &String) -> bool {
        let data_reader = self.container.write().await;
        data_reader.contains_key(query)
    }
    pub async fn remove(&self, query: &String) -> bool {
        let mut data_writer = self.container.write().await;
        data_writer.remove(query).is_some()
    }
    pub async fn list_all(&self)->HashMap<String,String>{
        let data_reader = self.container.read().await;
        data_reader.clone()
    }
}
