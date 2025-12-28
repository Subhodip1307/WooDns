use dashmap::DashMap;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct DockerStorage {
    container: Arc<DashMap<String, String>>,
}
impl DockerStorage {
    pub fn new() -> Self {
        // createing new instance of the storage
        Self {
            container: Arc::new(DashMap::new()),
        }
    }
    pub async fn get(&self, query: &String) -> Option<String> {
        if let Some(data) = self.container.get(query) {
            Some(data.value().clone())
        } else {
            self.container.get(&query.replace('_', "-")).map(|data| data.value().clone())
            }
        
    }
    pub async fn set(&self, query: String, value: String) {
        self.container.insert(query, value);
    }
    pub async fn contains(&self, query: &String) -> bool {
        dbg!(query);
        if self.container.contains_key(query) {
            true
        } else {
            self.container.contains_key(&query.replace('_', "-"))
        }
    }
    pub async fn remove(&self, query: &String) -> bool {
        self.container.remove(query);
        true
    }
    pub async fn list_all(&self) -> HashMap<String, String> {
        self.container
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}
