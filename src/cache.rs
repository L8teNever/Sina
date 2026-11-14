//! Kleiner Zwischenspeicher im Arbeitsspeicher (Schlüssel → JSON, mit Ablaufzeit).

use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};

use serde_json::Value;

#[derive(Default)]
pub struct Cache(Mutex<HashMap<String, (Instant, Value)>>);

impl Cache {
    pub fn get(&self, key: &str, ttl: Duration) -> Option<Value> {
        let map = self.0.lock().ok()?;
        let (at, v) = map.get(key)?;
        (at.elapsed() < ttl).then(|| v.clone())
    }

    pub fn put(&self, key: String, value: Value) {
        if let Ok(mut map) = self.0.lock() {
            if map.len() > 400 {
                let old: Vec<String> = map
                    .iter()
                    .filter(|(_, (at, _))| at.elapsed() > Duration::from_secs(1800))
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in old {
                    map.remove(&k);
                }
                if map.len() > 400 {
                    map.clear();
                }
            }
            map.insert(key, (Instant::now(), value));
        }
    }
}
