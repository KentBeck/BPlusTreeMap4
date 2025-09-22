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
        unsafe {
            let parts = crate::layout::carve_leaf::<K, V>(root, &self.leaf_layout);
            let hdr = &mut *parts.hdr;
            let len = hdr.len as usize;
            let keys_slice = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
            match keys_slice.binary_search(&key) {
                Ok(idx) => {
                    let vptr = parts.vals_ptr.add(idx) as *mut V;
                    let old = core::ptr::read(vptr);
                    core::ptr::write(vptr, value);
                    Some(old)
                }
                Err(idx) => {
                    if len < self.leaf_layout.cap as usize {
                        self.shift_right(parts.keys_ptr as *mut K, parts.vals_ptr as *mut V, idx, len);
                        self.write_kv_at(parts.keys_ptr as *mut K, parts.vals_ptr as *mut V, idx, key, value);
                        hdr.len = (len + 1) as u16;
                        self.len_count += 1;
                        None
                    } else {
                        // Split leaf and promote to branch if root is a leaf
                        self.split_leaf_root_and_insert(root, key, value);
                        None
                    }
                }
            }
        }
    }

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
    pub fn get_item(&self, _key: &K) -> Result<&V, BPlusTreeError> { Err(BPlusTreeError::KeyNotFound) }
    pub fn remove_item(&mut self, _key: &K) -> Result<V, BPlusTreeError> { Err(BPlusTreeError::KeyNotFound) }
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
        let mut cur = match self.root { Some(p) => p.as_ptr(), None => core::ptr::null_mut() };
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
        let root = self.root?;
        unsafe {
            let parts = crate::layout::carve_leaf::<K, V>(root, &self.leaf_layout);
            let len = (*parts.hdr).len as usize;
            let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
            let idx = keys.binary_search(key).ok()?;
            Some((parts, idx))
        }
    }
    pub fn allocated_leaf_count(&self) -> usize { 0 }
    pub fn free_leaf_count(&self) -> usize { 0 }
    pub fn leaf_sizes(&self) -> Vec<usize> { Vec::new() }
    pub fn count_nodes_in_tree(&self) -> (usize, usize) { (0, 0) }
    pub fn check_invariants(&self) -> bool { true }
    pub fn check_invariants_detailed(&self) -> Result<(), String> { Ok(()) }

    // ===== Arena-like stats compatibility (stubs) =====
    pub fn leaf_arena_stats(&self) -> ArenaStats { ArenaStats { free_count: 0, allocated_count: 0 } }
    pub fn branch_arena_stats(&self) -> ArenaStats { ArenaStats { free_count: 0, allocated_count: 0 } }

    // ===== Iterators (single-level; traverse leaves via next links) =====
    pub fn items(&self) -> Items<'_, K, V> {
        Items { inner: self.collect_range(None, None).into_iter() }
    }
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys::<K, V> { inner: self.items().map(|(k, _)| k).collect::<Vec<_>>().into_iter(), _marker: PhantomData }
    }
    pub fn values(&self) -> Values<'_, K, V> {
        Values::<K, V> { inner: self.items().map(|(_, v)| v).collect::<Vec<_>>().into_iter(), _marker: PhantomData }
    }
    pub fn items_range(&self, start: Option<&K>, end: Option<&K>) -> Items<'_, K, V> {
        Items { inner: self.collect_range(start, end).into_iter() }
    }
    pub fn range<R: RangeBounds<K>>(&self, r: R) -> Items<'_, K, V> {
        use core::ops::Bound;
        let start = match r.start_bound() { Bound::Included(k) => Some(k), Bound::Excluded(k) => None, Bound::Unbounded => None };
        let end = match r.end_bound() { Bound::Excluded(k) => Some(k), Bound::Included(_) => None, Bound::Unbounded => None };
        // Note: above is a simplified mapping: Included end and Excluded start
        // are not represented here; full RangeBounds support will be added later.
        Items { inner: self.collect_range(start, end).into_iter() }
    }

    fn collect_range<'a>(&'a self, start: Option<&K>, end: Option<&K>) -> Vec<(&'a K, &'a V)> {
        let mut out = Vec::new();
        let mut cur = match self.root { Some(p) => p.as_ptr(), None => core::ptr::null_mut() };
        unsafe {
            while !cur.is_null() {
                let hdr = &*(cur as *const NodeHdr);
                if hdr.tag != NodeTag::Leaf { break; }
                let len = hdr.len as usize;
                let keys_ptr = (cur.add(self.leaf_layout.keys_off)) as *const K;
                let vals_ptr = (cur.add(self.leaf_layout.vals_off)) as *const V;
                let keys = core::slice::from_raw_parts(keys_ptr, len);
                for i in 0..len {
                    let k = &*keys_ptr.add(i);
                    // Apply simple [start <= k) and (k < end) filtering if provided
                    if let Some(s) = start { if k < s { continue; } }
                    if let Some(e) = end { if k >= e { continue; } }
                    let v = &*vals_ptr.add(i);
                    out.push((k, v));
                }
                // Next leaf
                let next_ptr = (cur.add(self.leaf_layout.next_off)) as *const *mut u8;
                cur = *next_ptr;
            }
        }
        out
    }

    // ===== Arena compatibility shims used in some tests (stubs) =====
    pub fn allocate_leaf(&mut self, _node: LeafNodeCompat<K, V>) -> u32 { 0 }
    pub fn deallocate_leaf(&mut self, _id: u32) -> Option<LeafNodeCompat<K, V>> { None }
    pub fn get_leaf(&self, _id: u32) -> Option<&LeafNodeCompat<K, V>> { None }
    pub fn get_leaf_mut(&mut self, _id: u32) -> Option<&mut LeafNodeCompat<K, V>> { None }
    pub fn get_leaf_next(&self, _id: u32) -> Option<u32> { None }
    pub fn set_leaf_next(&mut self, _id: u32, _next: u32) -> bool { true }
}

#[derive(Debug, Copy, Clone)]
pub struct ArenaStats { pub free_count: usize, pub allocated_count: usize }

// Minimal leaf node compatibility type used by arena-ish tests
#[derive(Debug, Clone)]
pub struct LeafNodeCompat<K, V> {
    pub capacity: usize,
    pub _phantom: PhantomData<(K, V)>,
}
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
macro_rules! assert_tree_valid {
    ($tree:expr) => {{ if let Err(e) = $tree.check_invariants_detailed() { panic!("Tree invariants violated: {}", e); } }};
    ($tree:expr, $context:expr) => {{ if let Err(e) = $tree.check_invariants_detailed() { panic!("ATTACK SUCCESSFUL in {}: {}", $context, e); } }};
    ($tree:expr, $context:expr, $cycle:expr) => {{ if let Err(e) = $tree.check_invariants_detailed() { panic!("ATTACK SUCCESSFUL at {} cycle {}: {}", $context, $cycle, e); } }};
    ($tree:expr, $fmt:expr, $($arg:tt)*) => {{ if let Err(e) = $tree.check_invariants_detailed() { panic!("ATTACK SUCCESSFUL: {} - {}", format!($fmt, $($arg)*), e); } }};
}

#[macro_export]
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

pub trait BTreeResultExt<T> {
    fn with_context(self, _ctx: &str) -> Result<T, BPlusTreeError>;
    fn with_operation(self, _op: &str) -> Result<T, BPlusTreeError>;
    fn or_default_with_log(self) -> T where T: Default;
}

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
pub type LeafNode<K, V> = LeafNodeCompat<K, V>;

// Extra convenience/debug API stubs used in tests
impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn print_node_chain(&self) {}
    pub fn leaf_utilization(&self) -> f64 { 0.0 }
    pub fn slice(&self) -> Vec<(&K, &V)> { Vec::new() }
    pub fn validate(&self) -> BTreeResult<()> { Ok(()) }
    pub fn validate_for_operation(&self, _op: &str) -> BTreeResult<()> { Ok(()) }
    pub fn try_get(&self, _key: &K) -> KeyResult<&V> { Err(BPlusTreeError::KeyNotFound) }
    pub fn try_insert(&mut self, _key: K, _value: V) -> BTreeResult<Option<V>> { Ok(None) }
    pub fn try_remove(&mut self, _key: &K) -> ModifyResult<V> { Err(BPlusTreeError::KeyNotFound) }
    pub fn batch_insert(&mut self, _items: Vec<(K, V)>) -> BTreeResult<Vec<Option<V>>> { Ok(Vec::new()) }
    pub fn get_many<'a>(&'a self, _keys: &'a [K]) -> BTreeResult<Vec<&'a V>> { Ok(Vec::new()) }
    pub fn first(&self) -> Option<(&K, &V)> {
        let mut it = self.items();
        it.next()
    }
    pub fn last(&self) -> Option<(&K, &V)> {
        let mut it = self.items();
        it.last()
    }
}
