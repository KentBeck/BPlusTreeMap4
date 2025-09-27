use crate::{layout, BPlusTreeError, BPlusTreeMap};

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let (parts, idx) = self.leaf_search(key)?;
        unsafe {
            let value = core::ptr::read(parts.vals_ptr.add(idx) as *const V);
            let len = (*parts.hdr).len as usize;
            
            // Shift elements left to fill the gap
            if idx < len - 1 {
                core::ptr::copy(
                    parts.keys_ptr.add(idx + 1) as *const K,
                    parts.keys_ptr.add(idx) as *mut K,
                    len - idx - 1,
                );
                core::ptr::copy(
                    parts.vals_ptr.add(idx + 1) as *const V,
                    parts.vals_ptr.add(idx) as *mut V,
                    len - idx - 1,
                );
            }
            
            (*parts.hdr).len = (len - 1) as u16;
            self.len_count -= 1;
            Some(value)
        }
    }

    pub fn remove_item(&mut self, key: &K) -> Result<V, BPlusTreeError> {
        self.remove(key).ok_or(BPlusTreeError::KeyNotFound)
    }
}
