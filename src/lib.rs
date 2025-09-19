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
}

impl<K, V> BPlusTreeMap<K, V> {
    /// Common cache line size assumption (bytes).
    pub const CACHE_LINE_BYTES: usize = 64;

    /// Construct with explicit byte budgets for leaves and branches.
    /// Doubly-linked leaves are used to support reverse iteration efficiently.
    pub fn with_budgets(leaf_bytes: usize, branch_bytes: usize) -> Self {
        let leaf_layout = LeafLayout::compute::<K, V>(leaf_bytes, true);
        let branch_layout = BranchLayout::compute::<K>(branch_bytes);
        Self { root: None, leaf_layout, branch_layout, _marker: PhantomData }
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
// Public API surface (stubs)
// =============================

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

impl<K: Ord, V> BPlusTreeMap<K, V> {
    // ===== Compatibility constructors =====
    pub fn new(capacity: usize) -> Result<Self, BPlusTreeError> {
        if capacity < 4 {
            return Err(BPlusTreeError::InvalidCapacity("capacity too small".into()));
        }
        Ok(Self::with_cache_lines(32, 32))
    }

    pub fn is_empty(&self) -> bool { self.root.is_none() }
    pub fn len(&self) -> usize { 0 }
    pub fn clear(&mut self) {}

    // ===== Basic operations (stubs) =====
    pub fn insert(&mut self, _key: K, _value: V) -> Option<V> { None }
    pub fn get(&self, _key: &K) -> Option<&V> { None }
    pub fn get_mut(&mut self, _key: &K) -> Option<&mut V> { None }
    pub fn remove(&mut self, _key: &K) -> Option<V> { None }
    pub fn get_item(&self, _key: &K) -> Result<&V, BPlusTreeError> { Err(BPlusTreeError::KeyNotFound) }
    pub fn remove_item(&mut self, _key: &K) -> Result<V, BPlusTreeError> { Err(BPlusTreeError::KeyNotFound) }
    pub fn contains_key(&self, _key: &K) -> bool { false }
    pub fn get_or_default<'a>(&'a self, _key: &K, default: &'a V) -> &'a V { default }

    // ===== Structure/introspection (stubs) =====
    pub fn is_leaf_root(&self) -> bool { true }
    pub fn leaf_count(&self) -> usize { 1 }
    pub fn allocated_leaf_count(&self) -> usize { 0 }
    pub fn free_leaf_count(&self) -> usize { 0 }
    pub fn leaf_sizes(&self) -> Vec<usize> { Vec::new() }
    pub fn count_nodes_in_tree(&self) -> (usize, usize) { (0, 0) }
    pub fn check_invariants(&self) -> bool { true }
    pub fn check_invariants_detailed(&self) -> Result<(), String> { Ok(()) }

    // ===== Arena-like stats compatibility (stubs) =====
    pub fn leaf_arena_stats(&self) -> ArenaStats { ArenaStats { free_count: 0, allocated_count: 0 } }
    pub fn branch_arena_stats(&self) -> ArenaStats { ArenaStats { free_count: 0, allocated_count: 0 } }

    // ===== Iterators (empty stubs) =====
    pub fn items(&self) -> Items<'_, K, V> { Items { _marker: PhantomData } }
    pub fn keys(&self) -> Keys<'_, K, V> { Keys { _marker: PhantomData } }
    pub fn values(&self) -> Values<'_, K, V> { Values { _marker: PhantomData } }
    pub fn items_range(&self, _start: Option<&K>, _end: Option<&K>) -> Items<'_, K, V> { self.items() }
    pub fn range<R: RangeBounds<K>>(&self, _r: R) -> Items<'_, K, V> { self.items() }

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
// Iterators (stubs)
// ===============
pub struct Items<'a, K, V> { _marker: PhantomData<&'a (K, V)> }
impl<'a, K, V> Iterator for Items<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> { None }
}
impl<'a, K, V> DoubleEndedIterator for Items<'a, K, V> { fn next_back(&mut self) -> Option<<Self as Iterator>::Item> { None } }

pub struct Keys<'a, K, V> { _marker: PhantomData<&'a (K, V)> }
impl<'a, K, V> Iterator for Keys<'a, K, V> { type Item = &'a K; fn next(&mut self) -> Option<Self::Item> { None } }
impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> { fn next_back(&mut self) -> Option<Self::Item> { None } }

pub struct Values<'a, K, V> { _marker: PhantomData<&'a (K, V)> }
impl<'a, K, V> Iterator for Values<'a, K, V> { type Item = &'a V; fn next(&mut self) -> Option<Self::Item> { None } }
impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> { fn next_back(&mut self) -> Option<Self::Item> { None } }

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
impl<K: Ord, V> BPlusTreeMap<K, V> {
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
    pub fn first(&self) -> Option<(&K, &V)> { None }
    pub fn last(&self) -> Option<(&K, &V)> { None }
}
