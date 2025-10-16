use crate::{dealloc_raw, layout, BPlusTreeError, BPlusTreeMap, NodeHdr, NodeTag};
use core::ptr::{self, NonNull};

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let root = self.root?;
        let result = unsafe { self.remove_rec(root, key) };
        if result.is_some() {
            // Only check root collapse if root is a branch with few children
            // This avoids unnecessary checks when root is a leaf or has many children
            unsafe {
                if let Some(root) = self.root {
                    let hdr = &*(root.as_ptr() as *const NodeHdr);
                    if hdr.tag == NodeTag::Branch && (*hdr).len <= 2 {
                        self.check_root_collapse();
                    }
                }
            }
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
                        self.free_branch_node(root);
                    } else {
                        self.root = None;
                        self.free_branch_node(root);
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

        // Unlink from sibling chain
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

        // NOTE: We do NOT drop keys/values here because:
        // 1. Merge operations set len=0 after moving items out
        // 2. Items should already be dropped by the time we free the node
        // 3. Dropping here would cause double-free
        // The only exception is in free_tree_no_drop which handles cleanup differently

        dealloc_raw(leaf, self.leaf_layout.bytes, self.leaf_layout.max_align);
    }

    unsafe fn merge_leaf_into(&self, target: NonNull<u8>, source: NonNull<u8>) {
        let target_parts = layout::carve_leaf::<K, V>(target, &self.leaf_layout);
        let source_parts = layout::carve_leaf::<K, V>(source, &self.leaf_layout);

        let target_len = (*target_parts.hdr).len as usize;
        let source_len = (*source_parts.hdr).len as usize;

        // CRITICAL FIX: Check if merge would exceed capacity
        let merged_len = target_len + source_len;
        if merged_len > self.leaf_layout.cap as usize {
            // Cannot merge - would cause overflow. This should not happen if the
            // rebalancing algorithm is correct, but we must prevent corruption.
            panic!(
                "Leaf merge would exceed capacity: {} > {}",
                merged_len, self.leaf_layout.cap
            );
        }

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

    unsafe fn fix_branch_child(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let len = (*parts.hdr).len as usize;
        if len == 0 {
            return;
        }

        let children = parts.children_ptr as *mut *mut u8;
        let idx = child_idx.min(len);
        let child_ptr = *children.add(idx);
        let Some(_) = NonNull::new(child_ptr) else {
            return;
        };

        let child_hdr = &*(child_ptr as *const NodeHdr);
        match child_hdr.tag {
            NodeTag::Leaf => self.rebalance_leaf_child(branch, idx, len),
            NodeTag::Branch => self.rebalance_branch_child(branch, idx, len),
        }
    }

    unsafe fn rebalance_leaf_child(
        &mut self,
        branch: NonNull<u8>,
        child_idx: usize,
        branch_len: usize,
    ) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let children = parts.children_ptr as *mut *mut u8;

        let child_ptr = *children.add(child_idx);
        let child = NonNull::new_unchecked(child_ptr);
        let child_parts = layout::carve_leaf::<K, V>(child, &self.leaf_layout);
        let child_len = (*child_parts.hdr).len as usize;
        let min = self.min_leaf_len();
        if child_len >= min {
            return;
        }

        if child_idx > 0 {
            let left_ptr = *children.add(child_idx - 1);
            if let Some(left) = NonNull::new(left_ptr) {
                let left_hdr = &*(left_ptr as *const NodeHdr);
                if left_hdr.tag == NodeTag::Leaf {
                    let left_parts = layout::carve_leaf::<K, V>(left, &self.leaf_layout);
                    let left_len = (*left_parts.hdr).len as usize;
                    if left_len > min {
                        self.borrow_from_left_leaf(branch, child_idx);
                        return;
                    }
                }
            }
        }

        if child_idx < branch_len {
            let right_ptr = *children.add(child_idx + 1);
            if let Some(right) = NonNull::new(right_ptr) {
                let right_hdr = &*(right_ptr as *const NodeHdr);
                if right_hdr.tag == NodeTag::Leaf {
                    let right_parts = layout::carve_leaf::<K, V>(right, &self.leaf_layout);
                    let right_len = (*right_parts.hdr).len as usize;
                    if right_len > min {
                        self.borrow_from_right_leaf(branch, child_idx);
                        return;
                    }
                }
            }
        }

        if child_idx > 0 {
            self.merge_leaf_with_left(branch, child_idx);
        } else if child_idx < branch_len {
            self.merge_leaf_with_right(branch, child_idx);
        }
    }

    unsafe fn rebalance_branch_child(
        &mut self,
        branch: NonNull<u8>,
        child_idx: usize,
        branch_len: usize,
    ) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let children = parts.children_ptr as *mut *mut u8;

        let child_ptr = *children.add(child_idx);
        let child = NonNull::new_unchecked(child_ptr);
        let child_parts = layout::carve_branch::<K>(child, &self.branch_layout);
        let child_len = (*child_parts.hdr).len as usize;
        let min = self.min_branch_len();
        if child_len >= min {
            return;
        }

        if child_idx > 0 {
            let left_ptr = *children.add(child_idx - 1);
            if let Some(left) = NonNull::new(left_ptr) {
                let left_parts = layout::carve_branch::<K>(left, &self.branch_layout);
                let left_len = (*left_parts.hdr).len as usize;
                if left_len > min {
                    self.borrow_from_left_branch(branch, child_idx);
                    return;
                }
            }
        }

        if child_idx < branch_len {
            let right_ptr = *children.add(child_idx + 1);
            if let Some(right) = NonNull::new(right_ptr) {
                let right_parts = layout::carve_branch::<K>(right, &self.branch_layout);
                let right_len = (*right_parts.hdr).len as usize;
                if right_len > min {
                    self.borrow_from_right_branch(branch, child_idx);
                    return;
                }
            }
        }

        if child_idx > 0 {
            self.merge_branch_with_left(branch, child_idx);
        } else if child_idx < branch_len {
            self.merge_branch_with_right(branch, child_idx);
        }
    }

    unsafe fn borrow_from_left_branch(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let children = parts.children_ptr as *mut *mut u8;

        let left_ptr = *children.add(child_idx - 1);
        let child_ptr = *children.add(child_idx);
        let left = NonNull::new_unchecked(left_ptr);
        let child = NonNull::new_unchecked(child_ptr);

        let left_parts = layout::carve_branch::<K>(left, &self.branch_layout);
        let child_parts = layout::carve_branch::<K>(child, &self.branch_layout);

        let left_len = (*left_parts.hdr).len as usize;
        let child_len = (*child_parts.hdr).len as usize;

        let sep_slot = (parts.keys_ptr as *mut K).add(child_idx - 1);
        let parent_key = core::ptr::read(sep_slot);

        let left_keys = left_parts.keys_ptr as *mut K;
        let left_children = left_parts.children_ptr as *mut *mut u8;

        // CRITICAL FIX: Use safe move operation for the key
        let borrowed_key = core::ptr::read(left_keys.add(left_len - 1));
        core::ptr::write_bytes(left_keys.add(left_len - 1), 0, 1); // Clear source key slot

        let borrowed_child = *left_children.add(left_len);
        (*left_parts.hdr).len = (left_len - 1) as u16;
        *left_children.add(left_len) = ptr::null_mut();

        let child_keys = child_parts.keys_ptr as *mut K;
        let child_children = child_parts.children_ptr as *mut *mut u8;
        if child_len > 0 {
            core::ptr::copy(child_keys, child_keys.add(1), child_len);
        }
        core::ptr::copy(child_children, child_children.add(1), child_len + 1);
        core::ptr::write(child_keys, parent_key);
        *child_children.add(0) = borrowed_child;
        (*child_parts.hdr).len = (child_len + 1) as u16;

        core::ptr::write(sep_slot, borrowed_key);
    }

    unsafe fn borrow_from_right_branch(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let children = parts.children_ptr as *mut *mut u8;

        let child_ptr = *children.add(child_idx);
        let right_ptr = *children.add(child_idx + 1);
        let child = NonNull::new_unchecked(child_ptr);
        let right = NonNull::new_unchecked(right_ptr);

        let child_parts = layout::carve_branch::<K>(child, &self.branch_layout);
        let right_parts = layout::carve_branch::<K>(right, &self.branch_layout);

        let child_len = (*child_parts.hdr).len as usize;
        let right_len = (*right_parts.hdr).len as usize;

        let sep_slot = (parts.keys_ptr as *mut K).add(child_idx);
        let parent_key = core::ptr::read(sep_slot);

        let right_keys = right_parts.keys_ptr as *mut K;
        let right_children = right_parts.children_ptr as *mut *mut u8;

        // CRITICAL FIX: Use safe move operation for the key
        let new_sep = core::ptr::read(right_keys.add(0));
        core::ptr::write_bytes(right_keys.add(0), 0, 1); // Clear source key slot

        let transfer_child = *right_children.add(0);

        let child_keys = child_parts.keys_ptr as *mut K;
        let child_children = child_parts.children_ptr as *mut *mut u8;
        core::ptr::write(child_keys.add(child_len), parent_key);
        *child_children.add(child_len + 1) = transfer_child;
        (*child_parts.hdr).len = (child_len + 1) as u16;

        if right_len > 1 {
            core::ptr::copy(right_keys.add(1), right_keys, right_len - 1);
            // Clear the last key slot after shifting
            core::ptr::write_bytes(right_keys.add(right_len - 1), 0, 1);
        }
        core::ptr::copy(right_children.add(1), right_children, right_len);
        *right_children.add(right_len) = ptr::null_mut();
        (*right_parts.hdr).len = (right_len - 1) as u16;

        core::ptr::write(sep_slot, new_sep);
    }

    unsafe fn merge_branch_with_left(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let keys = parts.keys_ptr as *mut K;
        let children = parts.children_ptr as *mut *mut u8;

        let left_ptr = *children.add(child_idx - 1);
        let child_ptr = *children.add(child_idx);
        let left = NonNull::new_unchecked(left_ptr);
        let child = NonNull::new_unchecked(child_ptr);

        let left_parts = layout::carve_branch::<K>(left, &self.branch_layout);
        let child_parts = layout::carve_branch::<K>(child, &self.branch_layout);

        let left_len = (*left_parts.hdr).len as usize;
        let child_len = (*child_parts.hdr).len as usize;

        // CRITICAL FIX: Check if merge would exceed capacity
        let merged_len = left_len + 1 + child_len; // left + separator + child
        if merged_len > self.branch_layout.cap as usize {
            // Cannot merge - would cause overflow. This should not happen if the
            // rebalancing algorithm is correct, but we must prevent corruption.
            panic!(
                "Branch merge would exceed capacity: {} > {}",
                merged_len, self.branch_layout.cap
            );
        }

        let sep_slot = keys.add(child_idx - 1);
        let sep_key = core::ptr::read(sep_slot);

        let left_keys = left_parts.keys_ptr as *mut K;
        let left_children = left_parts.children_ptr as *mut *mut u8;
        let child_keys = child_parts.keys_ptr as *mut K;
        let child_children = child_parts.children_ptr as *mut *mut u8;

        core::ptr::write(left_keys.add(left_len), sep_key);
        for i in 0..child_len {
            core::ptr::write(
                left_keys.add(left_len + 1 + i),
                core::ptr::read(child_keys.add(i)),
            );
        }
        for i in 0..=child_len {
            *left_children.add(left_len + 1 + i) = *child_children.add(i);
        }
        (*left_parts.hdr).len = (left_len + 1 + child_len) as u16;
        (*child_parts.hdr).len = 0;

        self.free_branch_node(child);
        self.collapse_branch_entry(branch, child_idx - 1);
    }

    unsafe fn merge_branch_with_right(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let keys = parts.keys_ptr as *mut K;
        let children = parts.children_ptr as *mut *mut u8;

        let child_ptr = *children.add(child_idx);
        let right_ptr = *children.add(child_idx + 1);
        let child = NonNull::new_unchecked(child_ptr);
        let right = NonNull::new_unchecked(right_ptr);

        let child_parts = layout::carve_branch::<K>(child, &self.branch_layout);
        let right_parts = layout::carve_branch::<K>(right, &self.branch_layout);

        let child_len = (*child_parts.hdr).len as usize;
        let right_len = (*right_parts.hdr).len as usize;

        // CRITICAL FIX: Check if merge would exceed capacity
        let merged_len = child_len + 1 + right_len; // child + separator + right
        if merged_len > self.branch_layout.cap as usize {
            // Cannot merge - would cause overflow. This should not happen if the
            // rebalancing algorithm is correct, but we must prevent corruption.
            panic!(
                "Branch merge would exceed capacity: {} > {}",
                merged_len, self.branch_layout.cap
            );
        }

        let sep_slot = keys.add(child_idx);
        let sep_key = core::ptr::read(sep_slot);

        let child_keys = child_parts.keys_ptr as *mut K;
        let child_children = child_parts.children_ptr as *mut *mut u8;
        let right_keys = right_parts.keys_ptr as *mut K;
        let right_children = right_parts.children_ptr as *mut *mut u8;

        core::ptr::write(child_keys.add(child_len), sep_key);
        for i in 0..right_len {
            core::ptr::write(
                child_keys.add(child_len + 1 + i),
                core::ptr::read(right_keys.add(i)),
            );
        }
        for i in 0..=right_len {
            *child_children.add(child_len + 1 + i) = *right_children.add(i);
        }
        (*child_parts.hdr).len = (child_len + 1 + right_len) as u16;
        (*right_parts.hdr).len = 0;

        self.free_branch_node(right);
        self.collapse_branch_entry(branch, child_idx);
    }

    unsafe fn free_branch_node(&mut self, node: NonNull<u8>) {
        let parts = layout::carve_branch::<K>(node, &self.branch_layout);
        let len = (*parts.hdr).len as usize;

        // Drop all separator keys before deallocating
        for i in 0..len {
            ptr::drop_in_place((parts.keys_ptr as *mut K).add(i));
        }

        dealloc_raw(node, self.branch_layout.bytes, self.branch_layout.max_align);
    }

    unsafe fn collapse_branch_entry(&mut self, branch: NonNull<u8>, key_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let len = (*parts.hdr).len as usize;
        if key_idx >= len {
            return;
        }

        let keys = parts.keys_ptr as *mut K;
        let children = parts.children_ptr as *mut *mut u8;

        // Note: The key at key_idx has already been ptr::read out by the caller,
        // so we must not drop it here. We just shift the remaining keys.
        if key_idx < len - 1 {
            core::ptr::copy(keys.add(key_idx + 1), keys.add(key_idx), len - key_idx - 1);
        }
        // After shifting, the last key slot (at len-1) now contains a duplicate.
        // We must not drop it, so we'll rely on the length being decremented.

        core::ptr::copy(
            children.add(key_idx + 2),
            children.add(key_idx + 1),
            len - key_idx,
        );
        *children.add(len) = ptr::null_mut();
        (*parts.hdr).len = (len - 1) as u16;
    }

    unsafe fn borrow_from_left_leaf(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let keys = parts.keys_ptr as *mut K;
        let children = parts.children_ptr as *mut *mut u8;

        let left_ptr = *children.add(child_idx - 1);
        let child_ptr = *children.add(child_idx);
        let left = NonNull::new_unchecked(left_ptr);
        let child = NonNull::new_unchecked(child_ptr);

        let left_parts = layout::carve_leaf::<K, V>(left, &self.leaf_layout);
        let child_parts = layout::carve_leaf::<K, V>(child, &self.leaf_layout);

        let left_len = (*left_parts.hdr).len as usize;
        let child_len = (*child_parts.hdr).len as usize;

        // Shift child items right first to make room
        self.shift_right(
            child_parts.keys_ptr as *mut K,
            child_parts.vals_ptr as *mut V,
            0,
            child_len,
        );

        // CRITICAL FIX: Use safe move operation to transfer key-value from left to child
        // This ensures the source slot is properly cleared to prevent double-free
        self.move_kv_at(
            left_parts.keys_ptr as *mut K,
            left_parts.vals_ptr as *mut V,
            left_len - 1,
            child_parts.keys_ptr as *mut K,
            child_parts.vals_ptr as *mut V,
            0,
        );

        // Update lengths after the move
        (*left_parts.hdr).len = (left_len - 1) as u16;
        (*child_parts.hdr).len = (child_len + 1) as u16;

        let new_sep = self.key_clone_at(child_parts.keys_ptr as *const K, 0);
        let sep_slot = keys.add(child_idx - 1);
        let old_sep = core::ptr::read(sep_slot);
        drop(old_sep);
        core::ptr::write(sep_slot, new_sep);
    }

    unsafe fn borrow_from_right_leaf(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let keys = parts.keys_ptr as *mut K;
        let children = parts.children_ptr as *mut *mut u8;

        let child_ptr = *children.add(child_idx);
        let right_ptr = *children.add(child_idx + 1);
        let child = NonNull::new_unchecked(child_ptr);
        let right = NonNull::new_unchecked(right_ptr);

        let child_parts = layout::carve_leaf::<K, V>(child, &self.leaf_layout);
        let right_parts = layout::carve_leaf::<K, V>(right, &self.leaf_layout);

        let child_len = (*child_parts.hdr).len as usize;
        let right_len = (*right_parts.hdr).len as usize;

        // CRITICAL FIX: Use safe move operation to transfer key-value from right to child
        self.move_kv_at(
            right_parts.keys_ptr as *mut K,
            right_parts.vals_ptr as *mut V,
            0,
            child_parts.keys_ptr as *mut K,
            child_parts.vals_ptr as *mut V,
            child_len,
        );
        (*child_parts.hdr).len = (child_len + 1) as u16;

        // Shift remaining items in right leaf left to fill the gap
        // Since we used move_kv_at, the slot at index 0 is already cleared
        if right_len > 1 {
            core::ptr::copy(
                right_parts.keys_ptr.add(1) as *const K,
                right_parts.keys_ptr as *mut K,
                right_len - 1,
            );
            core::ptr::copy(
                right_parts.vals_ptr.add(1) as *const V,
                right_parts.vals_ptr as *mut V,
                right_len - 1,
            );
            // Clear the last slot to prevent stale data
            core::ptr::write_bytes(right_parts.keys_ptr.add(right_len - 1), 0, 1);
            core::ptr::write_bytes(right_parts.vals_ptr.add(right_len - 1), 0, 1);
        }
        // If right_len == 1, we've already transferred the only item, so nothing to drop
        (*right_parts.hdr).len = (right_len - 1) as u16;

        let new_sep = self.key_clone_at(right_parts.keys_ptr as *const K, 0);
        let sep_slot = keys.add(child_idx);
        let old_sep = core::ptr::read(sep_slot);
        drop(old_sep);
        core::ptr::write(sep_slot, new_sep);
    }

    unsafe fn merge_leaf_with_left(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let children = parts.children_ptr as *mut *mut u8;

        let left_ptr = *children.add(child_idx - 1);
        let child_ptr = *children.add(child_idx);
        let left = NonNull::new_unchecked(left_ptr);
        let child = NonNull::new_unchecked(child_ptr);

        self.merge_leaf_into(left, child);
        self.free_leaf_node(child);
        self.remove_branch_entry(branch, child_idx - 1);
    }

    unsafe fn merge_leaf_with_right(&mut self, branch: NonNull<u8>, child_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let children = parts.children_ptr as *mut *mut u8;

        let child_ptr = *children.add(child_idx);
        let right_ptr = *children.add(child_idx + 1);
        let child = NonNull::new_unchecked(child_ptr);
        let right = NonNull::new_unchecked(right_ptr);

        self.merge_leaf_into(child, right);
        self.free_leaf_node(right);
        self.remove_branch_entry(branch, child_idx);
    }

    unsafe fn remove_branch_entry(&mut self, branch: NonNull<u8>, key_idx: usize) {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let len = (*parts.hdr).len as usize;
        if key_idx >= len {
            return;
        }

        let keys = parts.keys_ptr as *mut K;
        let children = parts.children_ptr as *mut *mut u8;

        let removed = core::ptr::read(keys.add(key_idx));
        drop(removed);
        if key_idx < len - 1 {
            core::ptr::copy(keys.add(key_idx + 1), keys.add(key_idx), len - key_idx - 1);
        }

        core::ptr::copy(
            children.add(key_idx + 2),
            children.add(key_idx + 1),
            len - key_idx,
        );
        *children.add(len) = ptr::null_mut();
        (*parts.hdr).len = (len - 1) as u16;
    }

    unsafe fn remove_rec(&mut self, node: NonNull<u8>, key: &K) -> Option<V> {
        let hdr = &*(node.as_ptr() as *const NodeHdr);
        match hdr.tag {
            NodeTag::Leaf => self.leaf_remove(node, key),
            NodeTag::Branch => {
                let (child, idx) = self.child_for_key(node, key)?;
                let result = self.remove_rec(child, key);
                if result.is_some() {
                    self.fix_branch_child(node, idx);
                }
                result
            }
        }
    }

    unsafe fn leaf_remove(&mut self, leaf: NonNull<u8>, key: &K) -> Option<V> {
        let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
        let len = (*parts.hdr).len as usize;
        let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
        let idx = self.binary_search_keys(keys, key).ok()?;

        // Read the key and value (transferring ownership)
        let removed_key = core::ptr::read((parts.keys_ptr as *const K).add(idx));
        let value = core::ptr::read(parts.vals_ptr.add(idx) as *const V);

        // Shift remaining elements using batched operation
        if idx < len - 1 {
            self.shift_left_kv(
                parts.keys_ptr as *mut K,
                parts.vals_ptr as *mut V,
                idx,
                len - idx - 1,
            );
        }

        (*parts.hdr).len = (len - 1) as u16;

        // Drop the removed key (value is returned to caller)
        drop(removed_key);

        Some(value)
    }

    pub fn remove_item(&mut self, key: &K) -> Result<V, BPlusTreeError> {
        self.remove(key).ok_or(BPlusTreeError::KeyNotFound)
    }
}
