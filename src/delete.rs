use crate::{dealloc_raw, layout, BPlusTreeError, BPlusTreeMap, NodeHdr, NodeTag};
use core::ptr::{self, NonNull};

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
                    let child_count = len + 1;
                    let mut keep_child: Option<NonNull<u8>> = None;
                    let mut keep_is_leaf = false;

                    for i in 0..child_count {
                        let slot = parts.children_ptr.add(i) as *mut *mut u8;
                        let child_ptr = *slot;
                        if child_ptr.is_null() {
                            continue;
                        }

                        let child_hdr = &*(child_ptr as *const NodeHdr);
                        match child_hdr.tag {
                            NodeTag::Leaf => {
                                let child = NonNull::new_unchecked(child_ptr);
                                if (*child_hdr).len == 0 {
                                    self.free_leaf_node(child);
                                    *slot = ptr::null_mut();
                                    continue;
                                }
                                if let Some(existing) = keep_child {
                                    if !keep_is_leaf {
                                        return;
                                    }
                                    let existing_hdr = &*(existing.as_ptr() as *const NodeHdr);
                                    let existing_len = (*existing_hdr).len as usize;
                                    let child_len = (*child_hdr).len as usize;
                                    if existing_len + child_len > self.leaf_layout.cap as usize {
                                        return;
                                    }
                                    self.merge_leaf_into(existing, child);
                                    self.free_leaf_node(child);
                                    *slot = ptr::null_mut();
                                } else {
                                    keep_child = Some(child);
                                    keep_is_leaf = true;
                                }
                            }
                            NodeTag::Branch => {
                                if keep_child.is_some() {
                                    return;
                                }
                                keep_child = Some(NonNull::new_unchecked(child_ptr));
                                keep_is_leaf = false;
                            }
                        }
                    }

                    if let Some(child) = keep_child {
                        if keep_is_leaf {
                            self.make_leaf_root(child);
                        }
                        self.root = Some(child);
                        dealloc_raw(root, self.branch_layout.bytes, self.branch_layout.max_align);
                    } else {
                        self.root = None;
                        dealloc_raw(root, self.branch_layout.bytes, self.branch_layout.max_align);
                    }
                }
            }
        }
    }

    unsafe fn make_leaf_root(&self, leaf: NonNull<u8>) {
        let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
        if let Some(prev_ptr) = parts.prev_ptr {
            *prev_ptr = ptr::null_mut();
        }
    }

    unsafe fn free_leaf_node(&mut self, leaf: NonNull<u8>) {
        let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
        let next = *parts.next_ptr;
        let prev = match parts.prev_ptr {
            Some(prev_ptr) => *prev_ptr,
            None => ptr::null_mut(),
        };

        if !prev.is_null() {
            let prev_leaf = NonNull::new_unchecked(prev);
            let prev_parts = layout::carve_leaf::<K, V>(prev_leaf, &self.leaf_layout);
            *prev_parts.next_ptr = next;
        }

        if !next.is_null() {
            let next_leaf = NonNull::new_unchecked(next);
            let next_parts = layout::carve_leaf::<K, V>(next_leaf, &self.leaf_layout);
            if let Some(prev_ptr) = next_parts.prev_ptr {
                *prev_ptr = prev;
            }
        }

        *parts.next_ptr = ptr::null_mut();
        if let Some(prev_ptr) = parts.prev_ptr {
            *prev_ptr = ptr::null_mut();
        }

        dealloc_raw(leaf, self.leaf_layout.bytes, self.leaf_layout.max_align);
    }

    unsafe fn merge_leaf_into(&self, target: NonNull<u8>, source: NonNull<u8>) {
        let target_parts = layout::carve_leaf::<K, V>(target, &self.leaf_layout);
        let source_parts = layout::carve_leaf::<K, V>(source, &self.leaf_layout);

        let target_len = (*target_parts.hdr).len as usize;
        let source_len = (*source_parts.hdr).len as usize;

        let target_keys = target_parts.keys_ptr as *mut K;
        let target_vals = target_parts.vals_ptr as *mut V;
        let source_keys = source_parts.keys_ptr as *const K;
        let source_vals = source_parts.vals_ptr as *const V;

        for i in 0..source_len {
            let (key, val) = self.read_kv_at(source_keys, source_vals, i);
            self.write_kv_at(target_keys, target_vals, target_len + i, key, val);
        }

        (*target_parts.hdr).len = (target_len + source_len) as u16;
        (*source_parts.hdr).len = 0;
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
