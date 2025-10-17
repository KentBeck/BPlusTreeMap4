use alloc::format;
use alloc::string::String;
use core::ptr::NonNull;

use crate::layout;
use crate::{BPlusTreeMap, NodeHdr, NodeTag};

pub(crate) struct ValidationState<K> {
    pub(crate) total_items: usize,
    pub(crate) prev_leaf: Option<NonNull<u8>>,
    pub(crate) prev_key: Option<K>,
}

impl<K, V> BPlusTreeMap<K, V> {
    #[inline(always)]
    pub(crate) unsafe fn shift_right(
        &self,
        keys_ptr: *mut K,
        vals_ptr: *mut V,
        idx: usize,
        len: usize,
    ) {
        if idx < len {
            core::ptr::copy(keys_ptr.add(idx), keys_ptr.add(idx + 1), len - idx);
            core::ptr::copy(vals_ptr.add(idx), vals_ptr.add(idx + 1), len - idx);
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn write_kv_at(
        &self,
        keys_ptr: *mut K,
        vals_ptr: *mut V,
        idx: usize,
        key: K,
        val: V,
    ) {
        core::ptr::write(keys_ptr.add(idx), key);
        core::ptr::write(vals_ptr.add(idx), val);
    }

    #[inline(always)]
    pub(crate) unsafe fn write_key_at(&self, keys_ptr: *mut K, idx: usize, key: K) {
        core::ptr::write(keys_ptr.add(idx), key);
    }

    #[inline(always)]
    pub(crate) unsafe fn read_kv_at(
        &self,
        keys_ptr: *const K,
        vals_ptr: *const V,
        idx: usize,
    ) -> (K, V) {
        let k = core::ptr::read(keys_ptr.add(idx));
        let v = core::ptr::read(vals_ptr.add(idx));
        (k, v)
    }

    #[inline]
    pub(crate) unsafe fn key_clone_at(&self, keys_ptr: *const K, idx: usize) -> K
    where
        K: Clone,
    {
        (*keys_ptr.add(idx)).clone()
    }

    /// Centralized binary search for keys in a node.
    /// This function will be optimized for performance in future iterations.
    #[inline(always)]
    pub(crate) fn binary_search_keys<T: Ord>(
        &self,
        keys: &[T],
        target: &T,
    ) -> Result<usize, usize> {
        keys.binary_search(target)
    }

    /// Safely move a key-value pair from one location to another, ensuring sources are cleared.
    #[inline(always)]
    pub(crate) unsafe fn move_kv_at(
        &self,
        src_keys_ptr: *mut K,
        src_vals_ptr: *mut V,
        src_idx: usize,
        dst_keys_ptr: *mut K,
        dst_vals_ptr: *mut V,
        dst_idx: usize,
    ) {
        let key = core::ptr::read(src_keys_ptr.add(src_idx));
        let val = core::ptr::read(src_vals_ptr.add(src_idx));
        core::ptr::write(dst_keys_ptr.add(dst_idx), key);
        core::ptr::write(dst_vals_ptr.add(dst_idx), val);
        // Clear the source slots by writing zeros to prevent double-free
        core::ptr::write_bytes(src_keys_ptr.add(src_idx), 0, 1);
        core::ptr::write_bytes(src_vals_ptr.add(src_idx), 0, 1);
    }

    /// Shift key-value pairs left by one position (used after deletion).
    /// This batches the key and value copy operations together.
    #[inline(always)]
    pub(crate) unsafe fn shift_left_kv(
        &self,
        keys_ptr: *mut K,
        vals_ptr: *mut V,
        start_idx: usize,
        count: usize,
    ) {
        if count > 0 {
            core::ptr::copy(
                keys_ptr.add(start_idx + 1) as *const K,
                keys_ptr.add(start_idx) as *mut K,
                count,
            );
            core::ptr::copy(
                vals_ptr.add(start_idx + 1) as *const V,
                vals_ptr.add(start_idx) as *mut V,
                count,
            );
        }
    }
}

impl<K: Ord, V> BPlusTreeMap<K, V> {
    #[inline(always)]
    pub(crate) unsafe fn child_for_key(
        &self,
        branch: NonNull<u8>,
        key: &K,
    ) -> Option<(NonNull<u8>, usize)> {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let len = (*parts.hdr).len as usize;
        let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
        let child_idx = match self.binary_search_keys(keys, key) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        let child_ptr = *(parts.children_ptr.add(child_idx) as *const *mut u8);
        NonNull::new(child_ptr).map(|child| (child, child_idx))
    }

    #[inline(always)]
    pub(crate) fn leaf_for_key(&self, key: &K) -> Option<NonNull<u8>> {
        let mut cur = self.root?;
        unsafe {
            loop {
                let hdr = &*(cur.as_ptr() as *const NodeHdr);
                match hdr.tag {
                    NodeTag::Leaf => return Some(cur),
                    NodeTag::Branch => {
                        if let Some((child, _)) = self.child_for_key(cur, key) {
                            cur = child;
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
    }

    #[inline]
    pub(crate) fn leftmost_leaf(&self) -> Option<NonNull<u8>> {
        let mut cur = self.root?;
        unsafe {
            loop {
                let hdr = &*(cur.as_ptr() as *const NodeHdr);
                match hdr.tag {
                    NodeTag::Leaf => return Some(cur),
                    NodeTag::Branch => {
                        let b = layout::carve_branch::<K>(cur, &self.branch_layout);
                        let child_ptr = *(b.children_ptr as *const *mut u8);
                        if child_ptr.is_null() {
                            return None;
                        }
                        cur = NonNull::new_unchecked(child_ptr);
                    }
                }
            }
        }
    }
}

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    #[inline]
    pub(crate) fn rightmost_leaf(&self) -> Option<NonNull<u8>> {
        let mut cur = self.root?;
        unsafe {
            loop {
                let hdr = &*(cur.as_ptr() as *const NodeHdr);
                match hdr.tag {
                    NodeTag::Leaf => return Some(cur),
                    NodeTag::Branch => {
                        let b = layout::carve_branch::<K>(cur, &self.branch_layout);
                        let len = (*b.hdr).len as usize;
                        let child_ptr = *(b.children_ptr.add(len) as *const *mut u8);
                        if child_ptr.is_null() {
                            return None;
                        }
                        cur = NonNull::new_unchecked(child_ptr);
                    }
                }
            }
        }
    }

    pub fn is_leaf_root(&self) -> bool {
        match self.root {
            None => true,
            Some(p) => unsafe { (*((p.as_ptr()) as *const NodeHdr)).tag == NodeTag::Leaf },
        }
    }

    pub fn leaf_count(&self) -> usize {
        let mut count = 0usize;
        let mut cur = match self.leftmost_leaf() {
            Some(p) => p.as_ptr(),
            None => core::ptr::null_mut(),
        };
        unsafe {
            while !cur.is_null() {
                let hdr = &*(cur as *const NodeHdr);
                if hdr.tag != NodeTag::Leaf {
                    break;
                }
                count += 1;
                let next_ptr = (cur.add(self.leaf_layout.next_off)) as *const *mut u8;
                cur = *next_ptr;
            }
        }
        count
    }

    pub fn check_invariants(&self) -> bool {
        self.check_invariants_detailed().is_ok()
    }

    pub fn check_invariants_detailed(&self) -> Result<(), String> {
        let mut state = ValidationState {
            total_items: 0,
            prev_leaf: None,
            prev_key: None,
        };

        unsafe {
            match self.root {
                None => Ok(()),
                Some(root) => {
                    self.validate_node(root, None, None, true, &mut state)?;

                    if let Some(last_leaf) = state.prev_leaf {
                        let next_ptr =
                            *(last_leaf.as_ptr().add(self.leaf_layout.next_off) as *const *mut u8);
                        if !next_ptr.is_null() {
                            return Err("Tail leaf next pointer should be null".into());
                        }
                    }

                    Ok(())
                }
            }
        }
    }

    pub(crate) unsafe fn validate_node(
        &self,
        node: NonNull<u8>,
        lower: Option<&K>,
        upper: Option<&K>,
        is_root: bool,
        state: &mut ValidationState<K>,
    ) -> Result<Option<(K, K)>, String> {
        let hdr = &*(node.as_ptr() as *const NodeHdr);
        match hdr.tag {
            NodeTag::Leaf => self.validate_leaf(node, lower, upper, is_root, state),
            NodeTag::Branch => self.validate_branch(node, lower, upper, is_root, state),
        }
    }

    pub(crate) unsafe fn validate_leaf(
        &self,
        leaf: NonNull<u8>,
        lower: Option<&K>,
        upper: Option<&K>,
        is_root: bool,
        state: &mut ValidationState<K>,
    ) -> Result<Option<(K, K)>, String> {
        let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
        let hdr = &*parts.hdr;
        let len = hdr.len as usize;
        let cap = self.leaf_layout.cap as usize;

        if len > cap {
            return Err(format!("Leaf has {} keys but capacity is {}", len, cap));
        }

        if len == 0 {
            if is_root {
                return Ok(None);
            } else {
                return Err("Non-root leaf is empty".into());
            }
        }

        let min_required = self.min_leaf_len();
        if !is_root && len < min_required {
            return Err(format!(
                "Leaf underfull: has {} keys, minimum is {}",
                len, min_required
            ));
        }

        let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);

        for window in keys.windows(2) {
            if window[0] >= window[1] {
                return Err("Leaf keys not strictly increasing".into());
            }
        }

        if let Some(low) = lower {
            if keys[0] < *low {
                return Err("Leaf keys fall below lower bound".into());
            }
        }
        if let Some(high) = upper {
            if keys[len - 1] >= *high {
                return Err("Leaf keys exceed upper bound".into());
            }
        }

        if let Some(prev_leaf) = state.prev_leaf {
            let prev_next = *(prev_leaf.as_ptr().add(self.leaf_layout.next_off) as *const *mut u8);
            if prev_next != leaf.as_ptr() {
                return Err("Leaf next pointer mismatch".into());
            }
        }

        if let Some(prev_ptr) = parts.prev_ptr {
            match state.prev_leaf {
                Some(prev) => {
                    if *prev_ptr != prev.as_ptr() {
                        return Err("Leaf prev pointer mismatch".into());
                    }
                }
                None => {
                    if !(*prev_ptr).is_null() {
                        return Err("First leaf prev pointer should be null".into());
                    }
                }
            }
        }

        state.prev_leaf = Some(leaf);

        if let Some(prev_key) = &state.prev_key {
            if keys[0] <= *prev_key {
                return Err("Leaf keys not globally increasing".into());
            }
        }
        state.prev_key = Some(keys[len - 1].clone());
        state.total_items += len;

        Ok(Some((keys[0].clone(), keys[len - 1].clone())))
    }

    pub(crate) unsafe fn validate_branch(
        &self,
        branch: NonNull<u8>,
        lower: Option<&K>,
        upper: Option<&K>,
        is_root: bool,
        state: &mut ValidationState<K>,
    ) -> Result<Option<(K, K)>, String> {
        let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
        let len = (*parts.hdr).len as usize;
        let cap = self.branch_layout.cap as usize;

        if len > cap {
            return Err(format!("Branch has {} keys but capacity is {}", len, cap));
        }

        if len == 0 {
            if !is_root {
                return Err("Non-root branch has no keys".into());
            }
            let child_ptr = *(parts.children_ptr as *const *mut u8);
            if child_ptr.is_null() {
                return Ok(None);
            }
        }

        let min_required = self.min_branch_len();
        if !is_root && len < min_required {
            return Err(format!(
                "Branch underfull: has {} keys, minimum is {}",
                len, min_required
            ));
        }

        let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
        for window in keys.windows(2) {
            if window[0] >= window[1] {
                return Err("Branch keys not strictly increasing".into());
            }
        }

        if let Some(low) = lower {
            if len > 0 && keys[0] < *low {
                return Err("Branch keys fall below lower bound".into());
            }
        }
        if let Some(high) = upper {
            if len > 0 && keys[len - 1] >= *high {
                return Err("Branch keys exceed upper bound".into());
            }
        }

        let mut subtree_min: Option<K> = None;
        let mut subtree_max: Option<K> = None;

        for i in 0..=len {
            let child_ptr = *(parts.children_ptr.add(i) as *const *mut u8);
            let child = match NonNull::new(child_ptr) {
                Some(child) => child,
                None => return Err("Branch child pointer is null".into()),
            };

            let lower_bound = if i == 0 { lower } else { Some(&keys[i - 1]) };
            let upper_bound = if i == len { upper } else { Some(&keys[i]) };

            if let Some((child_min, child_max)) =
                self.validate_node(child, lower_bound, upper_bound, false, state)?
            {
                if subtree_min.is_none() {
                    subtree_min = Some(child_min.clone());
                }
                subtree_max = Some(child_max);
            }
        }

        Ok(match (subtree_min, subtree_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        })
    }

    #[inline(always)]
    pub(crate) fn min_leaf_len(&self) -> usize {
        let cap = self.leaf_layout.cap as usize;
        cap / 2
    }

    #[inline(always)]
    pub(crate) fn min_branch_len(&self) -> usize {
        let cap = self.branch_layout.cap as usize;
        if cap <= 2 {
            1
        } else {
            cap / 2
        }
    }
}
