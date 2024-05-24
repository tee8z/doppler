use std::collections::HashMap;
use std::hash::Hash;
pub struct CloneableHashMap<K, V> {
    inner_map: HashMap<K, V>,
}

impl<K, V> CloneableHashMap<K, V>
where
    K: PartialEq,
    K: Eq,
    K: Hash,
{
    pub fn new() -> Self {
        CloneableHashMap {
            inner_map: HashMap::new(),
        }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.inner_map.get(&key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner_map.insert(key, value)
    }
}

impl<K, V> Clone for CloneableHashMap<K, V>
where
    K: Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        CloneableHashMap {
            inner_map: self.inner_map.clone(),
        }
    }
}
