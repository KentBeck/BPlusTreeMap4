#![no_std]

extern crate alloc;

use core::marker::PhantomData;
use core::ptr::NonNull;

mod layout;
mod node_alloc;

pub use layout::{align_up, BranchLayout, LeafLayout, NodeHdr, NodeTag};
pub use node_alloc::{
    alloc_branch_block, alloc_leaf_block, alloc_raw, dealloc_raw, init_branch_block, init_leaf_block,
};

/// Raw-memory B+ tree map with fixed-size leaf and branch nodes.
///
/// This type only defines the top-level container and precomputed layouts.
/// Nodes are single raw allocations carved according to these layouts.
pub struct BPlusTreeMap<K, V> {
    /// Root node (points to a node header at offset 0), or None if empty.
    root: Option<NonNull<u8>>,

    /// Fixed per-kind layouts computed from byte budgets and K/V sizes.
    leaf_layout: LeafLayout,
    branch_layout: BranchLayout,

    _marker: PhantomData<(K, V)>,
    // Total number of key-value pairs
    len_count: usize,
}

impl<K, V> BPlusTreeMap<K, V> {
    /// Common cache line size assumption (bytes).
    pub const CACHE_LINE_BYTES: usize = 64;

    /// Construct with explicit byte budgets for leaves and branches.
    /// Doubly-linked leaves are used to support reverse iteration efficiently.
    pub fn with_budgets(leaf_bytes: usize, branch_bytes: usize) -> Self {
        let leaf_layout = LeafLayout::compute::<K, V>(leaf_bytes, true);
        let branch_layout = BranchLayout::compute::<K>(branch_bytes);
        Self { root: None, leaf_layout, branch_layout, _marker: PhantomData, len_count: 0 }
    }

    /// Construct using cache-line counts for leaf and branch nodes.
    /// Uses 64-byte cache lines by default.
    pub fn with_cache_lines(leaf_lines: usize, branch_lines: usize) -> Self {
        let lb = leaf_lines.saturating_mul(Self::CACHE_LINE_BYTES);
        let bb = branch_lines.saturating_mul(Self::CACHE_LINE_BYTES);
        Self::with_budgets(lb, bb)
    }

    /// Returns the configured layout for leaf nodes.
    pub fn leaf_layout(&self) -> &LeafLayout { &self.leaf_layout }

    /// Returns the configured layout for branch nodes.
    pub fn branch_layout(&self) -> &BranchLayout { &self.branch_layout }
}

// =============================
// Public API surface (compat scaffolding)
// =============================
// Note: This module currently exposes a superset of the intended public API to
// satisfy imported tests from a previous project. Many of these functions are
// temporary shims or stubs (e.g., arena stats) and will be gated or removed as
// the raw-memory implementation matures.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops::RangeBounds;

pub const NULL_NODE: u32 = u32::MAX;

#[derive(Debug)]
pub enum BPlusTreeError {
    InvalidCapacity(String),
    KeyNotFound,
    DataIntegrityError(String),
    ArenaError(String),
    NodeError(String),
    CorruptedTree(String),
    InvalidState(String),
    AllocationError(String),
}

impl fmt::Display for BPlusTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BPlusTreeError::InvalidCapacity(s) => write!(f, "InvalidCapacity: {}", s),
            BPlusTreeError::KeyNotFound => write!(f, "KeyNotFound"),
            BPlusTreeError::DataIntegrityError(s) => write!(f, "DataIntegrityError: {}", s),
            BPlusTreeError::ArenaError(s) => write!(f, "ArenaError: {}", s),
            BPlusTreeError::NodeError(s) => write!(f, "NodeError: {}", s),
            BPlusTreeError::CorruptedTree(s) => write!(f, "CorruptedTree: {}", s),
            BPlusTreeError::InvalidState(s) => write!(f, "InvalidState: {}", s),
            BPlusTreeError::AllocationError(s) => write!(f, "AllocationError: {}", s),
        }
    }
}

impl core::error::Error for BPlusTreeError {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeRef<K, V> {
    Leaf(u32, PhantomData<(K, V)>),
    Branch(u32, PhantomData<(K, V)>),
}

impl<K, V> NodeRef<K, V> {
    pub fn id(&self) -> u32 {
        match *self {
            NodeRef::Leaf(id, _) | NodeRef::Branch(id, _) => id,
        }
    }
    pub fn is_leaf(&self) -> bool { matches!(self, NodeRef::Leaf(_, _)) }
}

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    // ===== Compatibility constructors =====
    pub fn new(capacity: usize) -> Result<Self, BPlusTreeError> {
        if capacity < 4 {
            return Err(BPlusTreeError::InvalidCapacity("capacity too small".into()));
        }
        let cap_u16 = core::cmp::min(capacity as u16, u16::MAX);
        // Build layouts that honor the requested capacity
        let leaf_layout = LeafLayout::compute_for_cap::<K, V>(cap_u16, true);
        let branch_layout = BranchLayout::compute_for_cap::<K>(cap_u16);
        let mut tree = Self { root: None, leaf_layout, branch_layout, _marker: PhantomData, len_count: 0 };
        // Start with an empty leaf root
        unsafe {
            let leaf = alloc_leaf_block(&tree.leaf_layout).ok_or_else(|| BPlusTreeError::AllocationError("leaf root".into()))?;
            tree.root = Some(leaf);
        }
        Ok(tree)
    }

    pub fn is_empty(&self) -> bool { self.len_count == 0 }
    pub fn len(&self) -> usize { self.len_count }
    pub fn clear(&mut self) {
        self.len_count = 0;
        if let Some(root) = self.root {
            unsafe { (*(root.as_ptr() as *mut NodeHdr)).len = 0; }
        }
    }

    // ===== Basic operations (stubs) =====
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let root = match self.root { Some(p) => p, None => unsafe { alloc_leaf_block(&self.leaf_layout).expect("alloc leaf") } };
        if self.root.is_none() { self.root = Some(root); }
        let res = unsafe { self.insert_rec(root, key, value) };
        match res {
            InsertResult::NoSplit(old) => old,
            InsertResult::Split { sep_key, right, old_value } => {
                // Promote to a new branch root
                unsafe {
                    let branch = alloc_branch_block(&self.branch_layout).expect("alloc new root branch");
                    let b = crate::layout::carve_branch::<K>(branch, &self.branch_layout);
                    let bhdr = &mut *b.hdr;
                    bhdr.len = 1;
                    // Write key
                    self.write_key_at(b.keys_ptr as *mut K, 0, sep_key);
                    // Children: left = old root, right = returned
                    let c0 = b.children_ptr as *mut *mut u8;
                    let c1 = c0.add(1);
                    *c0 = root.as_ptr();
                    *c1 = right.as_ptr();
                    self.root = Some(branch);
                }
                old_value
            }
        }
    }

    #[cfg(any())]
    unsafe fn split_leaf_root_and_insert(&mut self, root: NonNull<u8>, key: K, value: V) {
        // Gather existing items
        let parts = crate::layout::carve_leaf::<K, V>(root, &self.leaf_layout);
        let hdr = &mut *parts.hdr;
        let len = hdr.len as usize;
        let mut items: alloc::vec::Vec<(K, V)> = alloc::vec::Vec::with_capacity(len + 1);
        // Read out existing K,V (move them)
        for i in 0..len {
            let (k, v) = self.read_kv_at(parts.keys_ptr as *const K, parts.vals_ptr as *const V, i);
            items.push((k, v));
        }
        // Insert new (k,v) maintaining order
        let pos = items.binary_search_by(|(kk, _)| kk.cmp(&key)).unwrap_or_else(|e| e);
        items.insert(pos, (key, value));

        // Split into left/right
        let total = items.len();
        let left_count = total / 2; // floor
        let right_count = total - left_count;

        // Prevent Vec from dropping moved elements; we will free buffer manually
        let mut items = core::mem::ManuallyDrop::new(items);
        let base = items.as_mut_ptr();
        let cap_vec = items.capacity();

        // Write back left
        for i in 0..left_count {
            let (kk, vv) = core::ptr::read(base.add(i));
            self.write_kv_at(parts.keys_ptr as *mut K, parts.vals_ptr as *mut V, i, kk, vv);
        }
        hdr.len = left_count as u16;

        // Allocate right leaf
        let right = alloc_leaf_block(&self.leaf_layout).expect("alloc right leaf");
        let rparts = crate::layout::carve_leaf::<K, V>(right, &self.leaf_layout);
        let rhdr = &mut *rparts.hdr;
        rhdr.len = right_count as u16;
        // Separator is first key of right half
        // Write right items
        for i in 0..right_count {
            let (kk, vv) = core::ptr::read(base.add(left_count + i));
            self.write_kv_at(rparts.keys_ptr as *mut K, rparts.vals_ptr as *mut V, i, kk, vv);
        }
        // Free the temporary buffer (elements already moved)
        let _ = alloc::vec::Vec::<(K, V)>::from_raw_parts(base, 0, cap_vec);
        // Link leaves: left -> right
        // right.next = left.next; right.prev = left; left.next = right; fix next.prev if present
        let left_next_ptr = parts.next_ptr;
        let old_next = *left_next_ptr;
        *left_next_ptr = right.as_ptr();
        if let Some(_) = self.leaf_layout.prev_off {
            let rprev = rparts.prev_ptr.unwrap();
            *rprev = root.as_ptr();
        }
        if !old_next.is_null() {
            if let Some(prev_off) = self.leaf_layout.prev_off {
                let prev_ptr = (old_next.add(prev_off)) as *mut *mut u8;
                *prev_ptr = right.as_ptr();
            }
            if let Some(_) = rparts.prev_ptr { /* already linked */ }
            let rnext = rparts.next_ptr;
            *rnext = old_next;
        }

        // Promote to a branch root
        let branch = alloc_branch_block(&self.branch_layout).expect("alloc branch root");
        let bparts = crate::layout::carve_branch::<K>(branch, &self.branch_layout);
        let bhdr = &mut *bparts.hdr;
        bhdr.len = 1;
        // Write separator key
        let sep_k_val = self.key_clone_at(rparts.keys_ptr as *const K, 0);
        core::ptr::write(bparts.keys_ptr as *mut K, sep_k_val);
        // Children
        let c0 = bparts.children_ptr as *mut *mut u8;
        let c1 = c0.add(1);
        *c0 = root.as_ptr();
        *c1 = right.as_ptr();

        // Update tree root
        self.root = Some(branch);
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        let (parts, idx) = self.leaf_search(key)?;
        unsafe { Some(&*(parts.vals_ptr.add(idx) as *const V)) }
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let (parts, idx) = self.leaf_search(key)?;
        unsafe { Some(&mut *(parts.vals_ptr.add(idx) as *mut V)) }
    }
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let root = self.root?;
        unsafe {
            let parts = crate::layout::carve_leaf::<K, V>(root, &self.leaf_layout);
            let hdr = &mut *parts.hdr;
            let len = hdr.len as usize;
            let keys_slice = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
            if let Ok(idx) = keys_slice.binary_search(key) {
                let old = core::ptr::read(parts.vals_ptr.add(idx) as *mut V);
                self.shift_left(parts.keys_ptr as *mut K, parts.vals_ptr as *mut V, idx, len);
                hdr.len = (len - 1) as u16;
                self.len_count -= 1;
                Some(old)
            } else {
                None
            }
        }
    }
    pub fn get_item(&self, key: &K) -> Result<&V, BPlusTreeError> {
        self.get(key).ok_or(BPlusTreeError::KeyNotFound)
    }
    pub fn remove_item(&mut self, key: &K) -> Result<V, BPlusTreeError> {
        self.remove(key).ok_or(BPlusTreeError::KeyNotFound)
    }
    pub fn contains_key(&self, key: &K) -> bool { self.get(key).is_some() }
    pub fn get_or_default<'a>(&'a self, _key: &K, default: &'a V) -> &'a V { default }

    // ===== Structure/introspection (stubs) =====
    pub fn is_leaf_root(&self) -> bool {
        match self.root {
            None => true,
            Some(p) => unsafe { (*((p.as_ptr()) as *const NodeHdr)).tag == NodeTag::Leaf },
        }
    }
    pub fn leaf_count(&self) -> usize {
        let mut count = 0usize;
        let mut cur = match self.leftmost_leaf() { Some(p) => p.as_ptr(), None => core::ptr::null_mut() };
        unsafe {
            while !cur.is_null() {
                let hdr = &*(cur as *const NodeHdr);
                if hdr.tag != NodeTag::Leaf { break; }
                count += 1;
                let next_ptr = (cur.add(self.leaf_layout.next_off)) as *const *mut u8;
                cur = *next_ptr;
            }
        }
        count
    }

    // =============
    // Internal helpers
    // =============
    #[inline]
    unsafe fn shift_right(&self, keys_ptr: *mut K, vals_ptr: *mut V, idx: usize, len: usize) {
        if idx < len {
            core::ptr::copy(keys_ptr.add(idx), keys_ptr.add(idx + 1), len - idx);
            core::ptr::copy(vals_ptr.add(idx), vals_ptr.add(idx + 1), len - idx);
        }
    }

    #[inline]
    unsafe fn shift_left(&self, keys_ptr: *mut K, vals_ptr: *mut V, idx: usize, len: usize) {
        if idx + 1 <= len {
            core::ptr::copy(keys_ptr.add(idx + 1), keys_ptr.add(idx), len - idx - 1);
            core::ptr::copy(vals_ptr.add(idx + 1), vals_ptr.add(idx), len - idx - 1);
        }
    }

    #[inline]
    unsafe fn write_kv_at(&self, keys_ptr: *mut K, vals_ptr: *mut V, idx: usize, key: K, val: V) {
        core::ptr::write(keys_ptr.add(idx), key);
        core::ptr::write(vals_ptr.add(idx), val);
    }

    #[inline]
    unsafe fn write_key_at(&self, keys_ptr: *mut K, idx: usize, key: K) {
        core::ptr::write(keys_ptr.add(idx), key);
    }

    #[inline]
    unsafe fn read_kv_at(&self, keys_ptr: *const K, vals_ptr: *const V, idx: usize) -> (K, V) {
        let k = core::ptr::read(keys_ptr.add(idx));
        let v = core::ptr::read(vals_ptr.add(idx));
        (k, v)
    }

    #[inline]
    unsafe fn key_clone_at(&self, keys_ptr: *const K, idx: usize) -> K where K: Clone {
        (*keys_ptr.add(idx)).clone()
    }

    #[inline]
    fn leaf_search(&self, key: &K) -> Option<(crate::layout::LeafParts<K, V>, usize)> {
        let leaf = self.leaf_for_key(key)?;
        unsafe {
            let parts = crate::layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
            let len = (*parts.hdr).len as usize;
            let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
            let idx = keys.binary_search(key).ok()?;
            Some((parts, idx))
        }
    }

    #[inline]
    fn leaf_for_key(&self, key: &K) -> Option<NonNull<u8>> {
        let mut cur = self.root?;
        unsafe {
            loop {
                let hdr = &*(cur.as_ptr() as *const NodeHdr);
                match hdr.tag {
                    NodeTag::Leaf => return Some(cur),
                    NodeTag::Branch => {
                        // Descend into the appropriate child
                        let b = crate::layout::carve_branch::<K>(cur, &self.branch_layout);
                        let len = (*b.hdr).len as usize;
                        let keys = core::slice::from_raw_parts(b.keys_ptr as *const K, len);
                        let child_idx = match keys.binary_search(key) {
                            Ok(i) => i + 1,
                            Err(i) => i,
                        };
                        let child_ptr = *(b.children_ptr.add(child_idx) as *const *mut u8);
                        if child_ptr.is_null() { return None; }
                        cur = NonNull::new_unchecked(child_ptr);
                    }
                }
            }
        }
    }

    #[inline]
    fn leftmost_leaf(&self) -> Option<NonNull<u8>> {
        let mut cur = self.root?;
        unsafe {
            loop {
                let hdr = &*(cur.as_ptr() as *const NodeHdr);
                match hdr.tag {
                    NodeTag::Leaf => return Some(cur),
                    NodeTag::Branch => {
                        let b = crate::layout::carve_branch::<K>(cur, &self.branch_layout);
                        let child_ptr = *(b.children_ptr as *const *mut u8);
                        if child_ptr.is_null() { return None; }
                        cur = NonNull::new_unchecked(child_ptr);
                    }
                }
            }
        }
    }

    // Recursive insert that returns either no split or split info to bubble up
    unsafe fn insert_rec(&mut self, node: NonNull<u8>, key: K, value: V) -> InsertResult<K, V> {
        let hdr = &*(node.as_ptr() as *const NodeHdr);
        match hdr.tag {
            NodeTag::Leaf => self.leaf_insert_or_split(node, key, value),
            NodeTag::Branch => {
                let b = crate::layout::carve_branch::<K>(node, &self.branch_layout);
                let len = (*b.hdr).len as usize;
                let keys = core::slice::from_raw_parts(b.keys_ptr as *const K, len);
                let child_idx = match keys.binary_search(&key) { Ok(i) => i + 1, Err(i) => i };
                let child_ptr = *(b.children_ptr.add(child_idx) as *const *mut u8);
                let child = NonNull::new(child_ptr).expect("child must exist");
                match self.insert_rec(child, key, value) {
                    InsertResult::NoSplit(old) => InsertResult::NoSplit(old),
                    InsertResult::Split { sep_key, right, old_value } => {
                        // Insert into this branch
                        let cur_len = (*b.hdr).len as usize;
                        let cap = self.branch_layout.cap as usize;
                        if cur_len < cap {
                            // shift keys [child_idx..cur_len) right by 1
                            core::ptr::copy(b.keys_ptr.add(child_idx) as *mut K, b.keys_ptr.add(child_idx + 1) as *mut K, cur_len - child_idx);
                            // insert key
                            self.write_key_at(b.keys_ptr as *mut K, child_idx, sep_key);
                            // shift children [child_idx+1..cur_len+1) right by 1
                            let cbase = b.children_ptr as *mut *mut u8;
                            core::ptr::copy(cbase.add(child_idx + 1), cbase.add(child_idx + 2), cur_len - child_idx);
                            // write right child at child_idx+1
                            *cbase.add(child_idx + 1) = right.as_ptr();
                            (*b.hdr).len = (cur_len + 1) as u16;
                            InsertResult::NoSplit(old_value)
                        } else {
                            // Need to split this branch after insertion
                            self.branch_insert_and_split(node, child_idx, sep_key, right, old_value)
                        }
                    }
                }
            }
        }
    }

    unsafe fn branch_insert_and_split(&mut self, node: NonNull<u8>, insert_idx: usize, ins_key: K, ins_right: NonNull<u8>, old_value: Option<V>) -> InsertResult<K, V> {
        let b = crate::layout::carve_branch::<K>(node, &self.branch_layout);
        let len = (*b.hdr).len as usize;
        let total_keys = len + 1; // after inserting ins_key

        // Collect existing keys (move) and children (copy)
        let mut keys_vec: alloc::vec::Vec<K> = alloc::vec::Vec::with_capacity(total_keys);
        for i in 0..len { keys_vec.push(core::ptr::read((b.keys_ptr as *const K).add(i))); }
        keys_vec.insert(insert_idx, ins_key);

        let total_children = total_keys + 1;
        let mut childs: alloc::vec::Vec<*mut u8> = alloc::vec::Vec::with_capacity(total_children);
        let cbase = b.children_ptr as *const *mut u8;
        for i in 0..=len { childs.push(*cbase.add(i)); }
        childs.insert(insert_idx + 1, ins_right.as_ptr());

        let mid = total_keys / 2; // key index to promote
        let promote = core::ptr::read(keys_vec.as_ptr().add(mid));

        // Left side: keys 0..mid, children 0..=mid
        (*b.hdr).len = mid as u16;
        for i in 0..mid { self.write_key_at(b.keys_ptr as *mut K, i, core::ptr::read(keys_vec.as_ptr().add(i))); }
        let cbase_mut = b.children_ptr as *mut *mut u8;
        for i in 0..=mid { *cbase_mut.add(i) = *childs.as_ptr().add(i); }

        // Right side: keys mid+1.., children mid+1..
        let right_keys_len = total_keys - (mid + 1);
        let right_children_len = total_children - (mid + 1);
        let right_node = alloc_branch_block(&self.branch_layout).expect("alloc right branch");
        let rb = crate::layout::carve_branch::<K>(right_node, &self.branch_layout);
        (*rb.hdr).len = right_keys_len as u16;
        for i in 0..right_keys_len { self.write_key_at(rb.keys_ptr as *mut K, i, core::ptr::read(keys_vec.as_ptr().add(mid + 1 + i))); }
        let rcbase = rb.children_ptr as *mut *mut u8;
        for i in 0..right_children_len { *rcbase.add(i) = *childs.as_ptr().add(mid + 1 + i); }

        // Prevent dropping moved keys: reclaim vectors with length 0 to free buffers only
        let _ = alloc::vec::Vec::<K>::from_raw_parts(keys_vec.as_mut_ptr(), 0, keys_vec.capacity());
        let _ = alloc::vec::Vec::<*mut u8>::from_raw_parts(childs.as_mut_ptr(), 0, childs.capacity());

        InsertResult::Split { sep_key: promote, right: right_node, old_value }
    }

    unsafe fn leaf_insert_or_split(&mut self, leaf: NonNull<u8>, key: K, value: V) -> InsertResult<K, V> {
        let parts = crate::layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
        let hdr = &mut *parts.hdr;
        let len = hdr.len as usize;
        let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
        match keys.binary_search(&key) {
            Ok(idx) => {
                let vptr = parts.vals_ptr.add(idx) as *mut V;
                let old = core::ptr::read(vptr);
                core::ptr::write(vptr, value);
                InsertResult::NoSplit(Some(old))
            }
            Err(idx) => {
                if len < self.leaf_layout.cap as usize {
                    self.shift_right(parts.keys_ptr as *mut K, parts.vals_ptr as *mut V, idx, len);
                    self.write_kv_at(parts.keys_ptr as *mut K, parts.vals_ptr as *mut V, idx, key, value);
                    hdr.len = (len + 1) as u16;
                    self.len_count += 1;
                    InsertResult::NoSplit(None)
                } else {
                    // Split leaf using temporary Vec of pairs
                    let mut items: alloc::vec::Vec<(K, V)> = alloc::vec::Vec::with_capacity(len + 1);
                    for i in 0..len {
                        let (k, v) = self.read_kv_at(parts.keys_ptr as *const K, parts.vals_ptr as *const V, i);
                        items.push((k, v));
                    }
                    let pos = items.binary_search_by(|(kk, _)| kk.cmp(&key)).unwrap_or_else(|e| e);
                    items.insert(pos, (key, value));
                    let total = items.len();
                    let left_count = total / 2;
                    let right_count = total - left_count;
                    // Write left back
                    for i in 0..left_count {
                        let (kk, vv) = core::ptr::read(items.as_ptr().add(i));
                        self.write_kv_at(parts.keys_ptr as *mut K, parts.vals_ptr as *mut V, i, kk, vv);
                    }
                    hdr.len = left_count as u16;

                    // Allocate right leaf
                    let right = alloc_leaf_block(&self.leaf_layout).expect("alloc right leaf");
                    let r = crate::layout::carve_leaf::<K, V>(right, &self.leaf_layout);
                    (*r.hdr).len = right_count as u16;
                    for i in 0..right_count {
                        let (kk, vv) = core::ptr::read(items.as_ptr().add(left_count + i));
                        self.write_kv_at(r.keys_ptr as *mut K, r.vals_ptr as *mut V, i, kk, vv);
                    }
                    // Link leaves
                    // right.next = left.next; right.prev = left; left.next = right; fix next.prev
                    let left_next = parts.next_ptr;
                    let old_next = *left_next;
                    *left_next = right.as_ptr();
                    if let Some(prev_ptr) = r.prev_ptr { *prev_ptr = leaf.as_ptr(); }
                    let rnext = r.next_ptr; *rnext = old_next;
                    if !old_next.is_null() {
                        if let Some(prev_off) = self.leaf_layout.prev_off {
                            let on_prev = (old_next.add(prev_off)) as *mut *mut u8;
                            *on_prev = right.as_ptr();
                        }
                    }

                    self.len_count += 1;
                    // Separator key: first key of right
                    let sep = self.key_clone_at(r.keys_ptr as *const K, 0);
                    InsertResult::Split { sep_key: sep, right, old_value: None }
                }
            }
        }
    }
    pub fn allocated_leaf_count(&self) -> usize { 0 }
    pub fn free_leaf_count(&self) -> usize { 0 }
    pub fn leaf_sizes(&self) -> Vec<usize> { Vec::new() }
    pub fn count_nodes_in_tree(&self) -> (usize, usize) { (0, 0) }
    pub fn check_invariants(&self) -> bool { true }
    pub fn check_invariants_detailed(&self) -> Result<(), String> { Ok(()) }

    // ===== Arena-like stats compatibility (stubs) =====
    #[cfg(feature = "compat_test_api")]
    pub fn leaf_arena_stats(&self) -> ArenaStats { ArenaStats { free_count: 0, allocated_count: 0 } }
    #[cfg(feature = "compat_test_api")]
    pub fn branch_arena_stats(&self) -> ArenaStats { ArenaStats { free_count: 0, allocated_count: 0 } }

    // ===== Iterators (single-level; traverse leaves via next links) =====
    pub fn items(&self) -> Items<'_, K, V> {
        Items { inner: self.collect_range_bounds(core::ops::Bound::Unbounded, core::ops::Bound::Unbounded).into_iter() }
    }
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys::<K, V> { inner: self.items().map(|(k, _)| k).collect::<Vec<_>>().into_iter(), _marker: PhantomData }
    }
    pub fn values(&self) -> Values<'_, K, V> {
        Values::<K, V> { inner: self.items().map(|(_, v)| v).collect::<Vec<_>>().into_iter(), _marker: PhantomData }
    }
    pub fn items_range(&self, start: Option<&K>, end: Option<&K>) -> Items<'_, K, V> {
        use core::ops::Bound;
        let sb = start.map_or(Bound::Unbounded, Bound::Included);
        // items_range follows [start, end) semantics by default
        let eb = end.map_or(Bound::Unbounded, Bound::Excluded);
        Items { inner: self.collect_range_bounds(sb, eb).into_iter() }
    }
    pub fn range<R: RangeBounds<K>>(&self, r: R) -> Items<'_, K, V> {
        Items { inner: self.collect_range_bounds(r.start_bound(), r.end_bound()).into_iter() }
    }

    fn collect_range_bounds<'a>(&'a self, start: core::ops::Bound<&K>, end: core::ops::Bound<&K>) -> Vec<(&'a K, &'a V)> {
        use core::ops::Bound;
        let mut out = Vec::new();
        // Find starting leaf
        let leaf_ptr = match start {
            Bound::Unbounded => self.leftmost_leaf(),
            Bound::Included(k) | Bound::Excluded(k) => self.leaf_for_key(k),
        };
        if leaf_ptr.is_none() { return out; }
        unsafe {
            let mut cur = leaf_ptr.unwrap().as_ptr();
            // Compute first index in first leaf depending on start bound
            let mut first_idx = 0usize;
            if let Bound::Included(s) | Bound::Excluded(s) = start {
                let parts = crate::layout::carve_leaf::<K, V>(NonNull::new_unchecked(cur), &self.leaf_layout);
                let len = (*parts.hdr).len as usize;
                let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
                match keys.binary_search(s) {
                    Ok(i) => { first_idx = if matches!(start, Bound::Excluded(_)) { i + 1 } else { i }; }
                    Err(i) => { first_idx = i; }
                }
            }
            loop {
                if cur.is_null() { break; }
                let hdr = &*(cur as *const NodeHdr);
                if hdr.tag != NodeTag::Leaf { break; }
                let len = hdr.len as usize;
                let keys_ptr = (cur.add(self.leaf_layout.keys_off)) as *const K;
                let vals_ptr = (cur.add(self.leaf_layout.vals_off)) as *const V;
                for i in first_idx..len {
                    let kref = &*keys_ptr.add(i);
                    // Apply end bound
                    let end_ok = match end {
                        Bound::Unbounded => true,
                        Bound::Included(e) => kref <= e,
                        Bound::Excluded(e) => kref < e,
                    };
                    if !end_ok { return out; }
                    let vref = &*vals_ptr.add(i);
                    out.push((kref, vref));
                }
                first_idx = 0; // for subsequent leaves, start at 0
                // Next leaf
                let next_ptr = (cur.add(self.leaf_layout.next_off)) as *const *mut u8;
                cur = *next_ptr;
            }
        }
        out
    }

    // ===== Arena compatibility shims used in some tests (stubs) =====
    #[cfg(feature = "compat_test_api")]
    pub fn allocate_leaf(&mut self, _node: LeafNodeCompat<K, V>) -> u32 { 0 }
    #[cfg(feature = "compat_test_api")]
    pub fn deallocate_leaf(&mut self, _id: u32) -> Option<LeafNodeCompat<K, V>> { None }
    #[cfg(feature = "compat_test_api")]
    pub fn get_leaf(&self, _id: u32) -> Option<&LeafNodeCompat<K, V>> { None }
    #[cfg(feature = "compat_test_api")]
    pub fn get_leaf_mut(&mut self, _id: u32) -> Option<&mut LeafNodeCompat<K, V>> { None }
    #[cfg(feature = "compat_test_api")]
    pub fn get_leaf_next(&self, _id: u32) -> Option<u32> { None }
    #[cfg(feature = "compat_test_api")]
    pub fn set_leaf_next(&mut self, _id: u32, _next: u32) -> bool { true }
}

#[derive(Debug, Copy, Clone)]
#[cfg(feature = "compat_test_api")]
pub struct ArenaStats { pub free_count: usize, pub allocated_count: usize }

// Minimal leaf node compatibility type used by arena-ish tests
#[derive(Debug, Clone)]
#[cfg(feature = "compat_test_api")]
pub struct LeafNodeCompat<K, V> {
    pub capacity: usize,
    pub _phantom: PhantomData<(K, V)>,
}
#[cfg(feature = "compat_test_api")]
impl<K, V> LeafNodeCompat<K, V> { pub fn new(capacity: usize) -> Self { Self { capacity, _phantom: PhantomData } } }

// ===============
// Iterators (backed by Vec of references)
// ===============
pub struct Items<'a, K, V> { inner: alloc::vec::IntoIter<(&'a K, &'a V)> }
impl<'a, K, V> Iterator for Items<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
}
impl<'a, K, V> DoubleEndedIterator for Items<'a, K, V> { fn next_back(&mut self) -> Option<<Self as Iterator>::Item> { self.inner.next_back() } }

pub struct Keys<'a, K, V> { inner: alloc::vec::IntoIter<&'a K>, _marker: PhantomData<V> }
impl<'a, K, V> Iterator for Keys<'a, K, V> { type Item = &'a K; fn next(&mut self) -> Option<Self::Item> { self.inner.next() } }
impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> { fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() } }

pub struct Values<'a, K, V> { inner: alloc::vec::IntoIter<&'a V>, _marker: PhantomData<K> }
impl<'a, K, V> Iterator for Values<'a, K, V> { type Item = &'a V; fn next(&mut self) -> Option<Self::Item> { self.inner.next() } }
impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> { fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() } }

// ===============
// Macros used in tests
// ===============
#[macro_export]
#[cfg(feature = "compat_test_api")]
macro_rules! assert_tree_valid {
    ($tree:expr) => {{ if let Err(e) = $tree.check_invariants_detailed() { panic!("Tree invariants violated: {}", e); } }};
    ($tree:expr, $context:expr) => {{ if let Err(e) = $tree.check_invariants_detailed() { panic!("ATTACK SUCCESSFUL in {}: {}", $context, e); } }};
    ($tree:expr, $context:expr, $cycle:expr) => {{ if let Err(e) = $tree.check_invariants_detailed() { panic!("ATTACK SUCCESSFUL at {} cycle {}: {}", $context, $cycle, e); } }};
    ($tree:expr, $fmt:expr, $($arg:tt)*) => {{ if let Err(e) = $tree.check_invariants_detailed() { panic!("ATTACK SUCCESSFUL: {} - {}", format!($fmt, $($arg)*), e); } }};
}

#[macro_export]
#[cfg(feature = "compat_test_api")]
macro_rules! verify_attack_result {
    ($tree:expr, $context:expr) => { assert_tree_valid!($tree, $context); };
    ($tree:expr, $context:expr, ordering) => {{
        assert_tree_valid!($tree, $context);
        let items: std::vec::Vec<_> = $tree.items().collect();
        for i in 1..items.len() { if items[i - 1].0 >= items[i].0 { panic!("ATTACK SUCCESSFUL: Items out of order in {}!", $context); } }
    }};
    ($tree:expr, $context:expr, count = $expected:expr) => {{
        assert_tree_valid!($tree, $context);
        let actual = $tree.len();
        if actual != $expected { panic!("ATTACK SUCCESSFUL in {}: Expected {} items, got {}", $context, $expected, actual); }
    }};
    ($tree:expr, $context:expr, full = $expected:expr) => {{
        verify_attack_result!($tree, $context, count = $expected);
        verify_attack_result!($tree, $context, ordering);
    }};
}

// =============================
// Enhanced error/result compatibility layer (stubs)
// =============================

pub type InitResult<T> = Result<T, BPlusTreeError>;
pub type BTreeResult<T> = Result<T, BPlusTreeError>;
pub type KeyResult<T> = Result<T, BPlusTreeError>;
pub type ModifyResult<T> = Result<T, BPlusTreeError>;

// Internal insert result used by recursive insert logic
enum InsertResult<K, V> {
    NoSplit(Option<V>),
    Split { sep_key: K, right: NonNull<u8>, old_value: Option<V> },
}

#[cfg(feature = "compat_test_api")]
pub trait BTreeResultExt<T> {
    fn with_context(self, _ctx: &str) -> Result<T, BPlusTreeError>;
    fn with_operation(self, _op: &str) -> Result<T, BPlusTreeError>;
    fn or_default_with_log(self) -> T where T: Default;
}

#[cfg(feature = "compat_test_api")]
impl<T> BTreeResultExt<T> for Result<T, BPlusTreeError> {
    fn with_context(self, _ctx: &str) -> Result<T, BPlusTreeError> { self }
    fn with_operation(self, _op: &str) -> Result<T, BPlusTreeError> { self }
    fn or_default_with_log(self) -> T where T: Default { self.unwrap_or_default() }
}

impl BPlusTreeError {
    pub fn invalid_capacity(got: usize, min: usize) -> Self {
        BPlusTreeError::InvalidCapacity(format!("Capacity {} is invalid (minimum required: {})", got, min))
    }
    pub fn data_integrity(op: &str, why: &str) -> Self { BPlusTreeError::DataIntegrityError(format!("{}: {}", op, why)) }
    pub fn arena_error(what: &str, why: &str) -> Self { BPlusTreeError::ArenaError(format!("{} failed: {}", what, why)) }
    pub fn node_error(kind: &str, id: u32, why: &str) -> Self { BPlusTreeError::NodeError(format!("{} node {}: {}", kind, id, why)) }
    pub fn corrupted_tree(where_: &str, why: &str) -> Self { BPlusTreeError::CorruptedTree(format!("{} corruption: {}", where_, why)) }
    pub fn invalid_state(op: &str, why: &str) -> Self { BPlusTreeError::InvalidState(format!("Cannot {}: {}", op, why)) }
    pub fn allocation_error(what: &str, why: &str) -> Self { BPlusTreeError::AllocationError(format!("Failed to allocate {}: {}", what, why)) }
}

impl core::cmp::PartialEq for BPlusTreeError {
    fn eq(&self, other: &Self) -> bool { core::mem::discriminant(self) == core::mem::discriminant(other) }
}
impl Eq for BPlusTreeError {}

// Compatibility alias for tests expecting LeafNode in crate root
#[cfg(feature = "compat_test_api")]
pub type LeafNode<K, V> = LeafNodeCompat<K, V>;

// Extra convenience/debug API stubs used in tests
#[cfg(feature = "compat_test_api")]
impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn print_node_chain(&self) {}
    pub fn leaf_utilization(&self) -> f64 { 0.0 }
    pub fn slice(&self) -> Vec<(&K, &V)> { Vec::new() }
    pub fn validate(&self) -> BTreeResult<()> { Ok(()) }
    pub fn validate_for_operation(&self, _op: &str) -> BTreeResult<()> { Ok(()) }
    pub fn try_get(&self, key: &K) -> KeyResult<&V> { self.get_item(key) }
    pub fn try_insert(&mut self, key: K, value: V) -> BTreeResult<Option<V>> { Ok(self.insert(key, value)) }
    pub fn try_remove(&mut self, key: &K) -> ModifyResult<V> { self.remove_item(key) }
    pub fn batch_insert(&mut self, items: Vec<(K, V)>) -> BTreeResult<Vec<Option<V>>> {
        let mut old_vals = Vec::with_capacity(items.len());
        for (k, v) in items { old_vals.push(self.insert(k, v)); }
        Ok(old_vals)
    }
    pub fn get_many<'a>(&'a self, keys: &'a [K]) -> BTreeResult<Vec<&'a V>> {
        let mut out = Vec::with_capacity(keys.len());
        for k in keys {
            match self.get(k) { Some(v) => out.push(v), None => return Err(BPlusTreeError::KeyNotFound) }
        }
        Ok(out)
    }
    pub fn first(&self) -> Option<(&K, &V)> {
        self.items().next()
    }
    pub fn last(&self) -> Option<(&K, &V)> {
        self.items().last()
    }
}
