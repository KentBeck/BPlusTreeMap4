use crate::{BPlusTreeError, BPlusTreeMap};

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn remove(&mut self, _key: &K) -> Option<V> {
        // TODO: implement delete operation
        None
    }

    pub fn remove_item(&mut self, key: &K) -> Result<V, BPlusTreeError> {
        self.remove(key).ok_or(BPlusTreeError::KeyNotFound)
    }
}
