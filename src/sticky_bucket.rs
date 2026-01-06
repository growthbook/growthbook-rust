use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::fmt::Debug;
use crate::model_public::GrowthBookAttribute;

pub trait StickyBucketService: Send + Sync + Debug {
    fn get_assignments(&self, attribute_name: &str, attribute_value: &str) -> Option<HashMap<String, String>>;
    fn save_assignments(&self, attribute_name: &str, attribute_value: &str, assignments: HashMap<String, String>);
    fn get_all_assignments(&self, attributes: &HashMap<String, GrowthBookAttribute>) -> HashMap<String, String>;
}

#[derive(Debug, Default)]
pub struct InMemoryStickyBucketService {
    // Key: $"{attribute_name}||${attribute_value}" -> Value: HashMap<doc_key, assignment>
    storage: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
}

impl InMemoryStickyBucketService {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn get_key(&self, attribute_name: &str, attribute_value: &str) -> String {
        format!("{}||{}", attribute_name, attribute_value)
    }
}

impl StickyBucketService for InMemoryStickyBucketService {
    fn get_assignments(&self, attribute_name: &str, attribute_value: &str) -> Option<HashMap<String, String>> {
        let storage = self.storage.read().unwrap();
        let key = self.get_key(attribute_name, attribute_value);
        storage.get(&key).cloned()
    }

    fn save_assignments(&self, attribute_name: &str, attribute_value: &str, assignments: HashMap<String, String>) {
        let mut storage = self.storage.write().unwrap();
        let key = self.get_key(attribute_name, attribute_value);
        
        // Merge with existing assignments
        let entry = storage.entry(key).or_default();
        entry.extend(assignments);
    }

    fn get_all_assignments(&self, attributes: &HashMap<String, GrowthBookAttribute>) -> HashMap<String, String> {
        let storage = self.storage.read().unwrap();
        let mut all_assignments = HashMap::new();

        for (attr_name, attr_value) in attributes {
            let key = self.get_key(attr_name, &attr_value.value.to_string());
            if let Some(assignments) = storage.get(&key) {
                all_assignments.extend(assignments.clone());
            }
        }
        all_assignments
    }
}
