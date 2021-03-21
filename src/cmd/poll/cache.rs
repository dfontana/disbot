use std::{collections::HashMap, hash::Hash, sync::RwLock};

pub struct Cache<K: Eq + Hash, V> {
  cache: RwLock<HashMap<K, V>>,
}

impl<K: Eq + Hash, V> Cache<K, V> {
  pub fn new() -> Cache<K, V> {
    Cache {
      cache: RwLock::new(HashMap::new()),
    }
  }

  pub fn contains_key(&self, key: &K) -> Result<bool, String> {
    match self.cache.read() {
      Err(e) => Err(format!("Failed to aquire lock - {}", e)),
      Ok(lock) => Ok(lock.contains_key(key)),
    }
  }

  pub fn insert(&self, key: K, value: V) -> Result<(), String> {
    match self.cache.write() {
      Err(e) => Err(format!("Failed to aquire lock - {}", e)),
      Ok(mut lock) => {
        lock.insert(key, value);
        Ok(())
      }
    }
  }

  pub fn invoke<F, T>(&self, id: &K, apply: F) -> Result<T, String>
  where
    F: FnOnce(&V) -> T,
  {
    match self.cache.read() {
      Err(e) => Err(format!("Failed to aquire lock - {}", e)),
      Ok(lock) => match lock.get(id) {
        None => Err("Key does not exist in cache to invoke".into()),
        Some(v) => Ok(apply(v)),
      },
    }
  }

  pub fn invoke_mut<F>(&self, key: &K, mut apply: F) -> Result<(), String>
  where
    F: FnMut(&mut V),
  {
    match self.cache.write() {
      Err(e) => Err(format!("Failed to aquire lock - {}", e)),
      Ok(mut lock) => match lock.get_mut(key) {
        None => Err("Key does not exist in cache to invoke".into()),
        Some(v) => Ok(apply(v)),
      },
    }
  }
}
