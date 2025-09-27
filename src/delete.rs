use crate::{layout, BPlusTreeError, BPlusTreeMap, NodeHdr, NodeTag};
use core::ptr::NonNull;

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let root = self.root?;
        let result = unsafe { self.remove_rec(root, key) };
        if result.is_some() {
            unsafe { self.check_root_collapse() };
        }
        result
    }

    unsafe fn check_root_collapse(&mut self) {
        if let Some(root) = self.root {
            let hdr = &*(root.as_ptr() as *const NodeHdr);
            if hdr.tag == NodeTag::Branch {
                let parts = layout::carve_branch::<K>(root, &self.branch_layout);
                let len = (*parts.hdr).len as usize;
                if len <= 1 {
                    let child_ptr = *(parts.children_ptr as *const *mut u8);
                    if !child_ptr.is_null() {
                        let child_hdr = &*(child_ptr as *const NodeHdr);
                        if child_hdr.tag == NodeTag::Leaf {
                            self.root = NonNull::new(child_ptr);
                        }
                    }
                }
            }
        }
    }

    unsafe fn remove_rec(&mut self, node: NonNull<u8>, key: &K) -> Option<V> {
        let hdr = &*(node.as_ptr() as *const NodeHdr);
        match hdr.tag {
            NodeTag::Leaf => self.leaf_remove(node, key),
            NodeTag::Branch => {
                let (child, _) = self.child_for_key(node, key)?;
                self.remove_rec(child, key)
            }
        }
    }

    unsafe fn leaf_remove(&mut self, leaf: NonNull<u8>, key: &K) -> Option<V> {
        let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
        let len = (*parts.hdr).len as usize;
        let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
        let idx = keys.binary_search(key).ok()?;
        
        let value = core::ptr::read(parts.vals_ptr.add(idx) as *const V);
        
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

    pub fn remove_item(&mut self, key: &K) -> Result<V, BPlusTreeError> {
        self.remove(key).ok_or(BPlusTreeError::KeyNotFound)
    }
}
