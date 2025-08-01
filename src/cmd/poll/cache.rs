use anyhow::anyhow;
use std::{
  collections::HashMap,
  hash::Hash,
  sync::RwLock,
  time::{Duration, Instant},
};

pub struct Cache<K: Eq + Hash + Clone, V: Expiring> {
  cache: RwLock<HashMap<K, Timestamped<V>>>,
}

pub trait Expiring {
  fn duration(&self) -> Duration;
}

struct Timestamped<V> {
  val: V,
  creation: Instant,
}

impl<V> Timestamped<V> {
  pub fn new(val: V) -> Timestamped<V> {
    Timestamped {
      val,
      creation: Instant::now(),
    }
  }

  pub fn expired(&self, expiration: &Duration) -> bool {
    Instant::now().duration_since(self.creation) >= *expiration
  }
}

impl<K: Eq + Hash + Clone, V: Expiring> Cache<K, V> {
  pub fn new() -> Cache<K, V> {
    Cache {
      cache: RwLock::new(HashMap::new()),
    }
  }

  pub fn contains_key(&self, key: &K) -> Result<bool, anyhow::Error> {
    match self.cache.read() {
      Err(e) => Err(anyhow!("Failed to aquire lock - {}", e)),
      Ok(lock) => Ok(lock.contains_key(key)),
    }
  }

  pub fn insert(&self, key: K, value: V) -> Result<(), anyhow::Error> {
    match self.cache.write() {
      Err(e) => Err(anyhow!("Failed to aquire lock - {}", e)),
      Ok(mut lock) => {
        let exp = &value.duration().clone();
        lock.insert(key, Timestamped::new(value));

        // Reap expired items
        let drop_keys: Vec<K> = lock
          .iter()
          .filter_map(|(k, v)| match v.expired(exp) {
            true => Some(k.clone()),
            false => None,
          })
          .collect();
        drop_keys.iter().for_each(|k| {
          lock.remove(k);
        });
        Ok(())
      }
    }
  }

  pub fn remove(&self, key: &K) -> Result<(), anyhow::Error> {
    match self.cache.write() {
      Err(e) => Err(anyhow!("Failed to aquire lock - {}", e)),
      Ok(mut lock) => {
        lock.remove(key);
        Ok(())
      }
    }
  }

  pub fn invoke<F, T>(&self, id: &K, apply: F) -> Result<T, anyhow::Error>
  where
    F: FnOnce(&V) -> T,
  {
    match self.cache.read() {
      Err(e) => Err(anyhow!("Failed to aquire lock - {}", e)),
      Ok(lock) => match lock.get(id) {
        None => Err(anyhow!("Key does not exist in cache to invoke")),
        Some(v) => Ok(apply(&v.val)),
      },
    }
  }

  pub fn invoke_mut<F>(&self, key: &K, mut apply: F) -> Result<(), anyhow::Error>
  where
    F: FnMut(&mut V),
  {
    match self.cache.write() {
      Err(e) => Err(anyhow!("Failed to aquire lock - {}", e)),
      Ok(mut lock) => match lock.get_mut(key) {
        None => Err(anyhow!("Key does not exist in cache to invoke")),
        Some(v) => {
          apply(&mut v.val);
          Ok(())
        }
      },
    }
  }

  pub fn iter<F, R>(&self, apply: F) -> Result<Vec<R>, anyhow::Error>
  where
    F: Fn(&K, &V) -> R,
  {
    match self.cache.read() {
      Err(e) => Err(anyhow!("Failed to acquire lock - {}", e)),
      Ok(lock) => Ok(
        lock
          .iter()
          .map(|(k, timestamped)| apply(k, &timestamped.val))
          .collect(),
      ),
    }
  }
}
