use alloc::vec::Vec;

use crate::layout;
use crate::{BPlusTreeError, BPlusTreeMap, BTreeResult};

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        let (parts, idx) = self.leaf_search(key)?;
        unsafe { Some(&*(parts.vals_ptr.add(idx) as *const V)) }
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let (parts, idx) = self.leaf_search(key)?;
        unsafe { Some(&mut *(parts.vals_ptr.add(idx) as *mut V)) }
    }

    pub fn get_item(&self, key: &K) -> Result<&V, BPlusTreeError> {
        self.get(key).ok_or(BPlusTreeError::KeyNotFound)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    pub fn get_or_default<'a>(&'a self, key: &K, default: &'a V) -> &'a V {
        self.get(key).unwrap_or(default)
    }

    pub fn get_many<'a>(&'a self, keys: &'a [K]) -> BTreeResult<Vec<&'a V>> {
        let mut out = Vec::with_capacity(keys.len());
        for k in keys {
            match self.get(k) {
                Some(v) => out.push(v),
                None => return Err(BPlusTreeError::KeyNotFound),
            }
        }
        Ok(out)
    }

    pub(crate) fn leaf_search(&self, key: &K) -> Option<(layout::LeafParts<K, V>, usize)> {
        let leaf = self.leaf_for_key(key)?;
        unsafe {
            let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
            let len = (*parts.hdr).len as usize;
            let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
            let idx = self.binary_search_keys(keys, key).ok()?;
            Some((parts, idx))
        }
    }
}
