//! Simple key->value cache with defined time to live

use std::{
    collections::HashMap,
    hash::Hash,
    time::Instant,
};

// Maybe TODO: Cache ready to use as shared in tokio loop
// pub(super) type SharedCache<V> = Rc<RwLock<CacheValue<V>>>;
// pub(super) type SharedCacheMap<K, V> = Rc<RwLock<HashMap<K, CacheValue<V>>>>;

// TODO: Unit-tests needed

pub struct CacheMap<K, V> {
    values: HashMap<K, CacheValue<V>>,
    ttl: u64,
}

#[derive(Clone)]
pub struct CacheValue<T> {
    pub value: Option<T>,
    timestamp: Instant,
    first_run: bool,
}

pub struct Entry<'a, K, V> {
    pub value: &'a CacheValue<V>,
    container: &'a CacheMap<K, V>
}

impl<K: Hash + Eq, V> CacheMap<K,V> {
    pub fn new(ttl: u64) -> Self {
        Self {
            values: Default::default(),
            ttl,
        }
    }

    pub fn should_retain(&self, key: &K) -> Option<bool> {
        self.values.get(key)
            .map(|val| val.age() < self.ttl)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.values.get(key)
            .and_then(|val| val.value.as_ref())
    }

    pub fn entry<'a>(&'a self, key: &'a K) -> Option<Entry<'a, K, V>> {
        self.values.get(key)
            .map(|value|
                Entry {
                    value,
                    container: self,
                }
            )
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<CacheValue<V>> {
        self.values.insert(key, CacheValue::from(Some(value)))
    }
}

impl<K, V> Entry<'_, K, V> {
    pub fn is_fresh(&self) -> bool {
        self.value.is_fresh(self.container.ttl)
    }

    pub fn should_retain(&self) -> bool {
        self.value.age() < self.container.ttl
    }
}

impl<T> CacheValue<T> {
    pub fn age(&self) -> u64 {
        self.timestamp.elapsed().as_secs()
    }

    pub fn is_fresh(&self, ttl: u64) -> bool {
        !self.first_run && self.age() < ttl
    }
}

impl<T> Default for CacheValue<T> {
    fn default() -> Self {
        Self {
            value: None,
            timestamp: Instant::now(),
            first_run: true,
        }
    }
}

impl<T> From<Option<T>> for CacheValue<T> {
    fn from(value: Option<T>) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
            first_run: false,
        }
    }
}
