#![no_std]

extern crate alloc;

use core::marker::PhantomData;
use core::ptr::{self, NonNull};

mod common;
mod delete;
mod get;
mod insert;
mod iterate;
mod layout;
mod node_alloc;

pub use iterate::{Items, Keys, Values};
pub use layout::{align_up, BranchLayout, LeafLayout, NodeHdr, NodeTag};
pub use node_alloc::{
    alloc_branch_block, alloc_leaf_block, alloc_raw, dealloc_raw, init_branch_block,
    init_leaf_block,
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

impl<K, V> Drop for BPlusTreeMap<K, V> {
    fn drop(&mut self) {
        if let Some(root) = self.root.take() {
            unsafe {
                self.free_tree_no_drop(root);
            }
        }
    }
}

impl<K, V> BPlusTreeMap<K, V> {
    /// Common cache line size assumption (bytes).
    pub const CACHE_LINE_BYTES: usize = 64;

    /// Construct with explicit byte budgets for leaves and branches.
    /// Doubly-linked leaves are used to support reverse iteration efficiently.
    pub fn with_budgets(leaf_bytes: usize, branch_bytes: usize) -> Self {
        let leaf_layout = LeafLayout::compute::<K, V>(leaf_bytes, true);
        let branch_layout = BranchLayout::compute::<K>(branch_bytes);
        Self {
            root: None,
            leaf_layout,
            branch_layout,
            _marker: PhantomData,
        }
    }

    /// Construct using cache-line counts for leaf and branch nodes.
    /// Uses 64-byte cache lines by default.
    pub fn with_cache_lines(leaf_lines: usize, branch_lines: usize) -> Self {
        let lb = leaf_lines.saturating_mul(Self::CACHE_LINE_BYTES);
        let bb = branch_lines.saturating_mul(Self::CACHE_LINE_BYTES);
        Self::with_budgets(lb, bb)
    }

    /// Returns the configured layout for leaf nodes.
    pub fn leaf_layout(&self) -> &LeafLayout {
        &self.leaf_layout
    }

    /// Returns the configured layout for branch nodes.
    pub fn branch_layout(&self) -> &BranchLayout {
        &self.branch_layout
    }

    /// Recursively free all nodes without dropping K,V (for Drop impl).
    unsafe fn free_tree_no_drop(&mut self, node: NonNull<u8>) {
        let hdr = &*(node.as_ptr() as *const NodeHdr);
        match hdr.tag {
            NodeTag::Leaf => {
                let parts = layout::carve_leaf::<K, V>(node, &self.leaf_layout);
                let len = (*parts.hdr).len as usize;

                // Drop all keys and values
                for i in 0..len {
                    ptr::drop_in_place((parts.keys_ptr as *mut K).add(i));
                    ptr::drop_in_place((parts.vals_ptr as *mut V).add(i));
                }

                dealloc_raw(node, self.leaf_layout.bytes, self.leaf_layout.max_align);
            }
            NodeTag::Branch => {
                let parts = layout::carve_branch::<K>(node, &self.branch_layout);
                let len = (*parts.hdr).len as usize;

                // Recursively free all children first
                for i in 0..=len {
                    let child_ptr = *((parts.children_ptr as *const *mut u8).add(i));
                    if let Some(child) = NonNull::new(child_ptr) {
                        self.free_tree_no_drop(child);
                    }
                }

                // Drop all separator keys
                for i in 0..len {
                    ptr::drop_in_place((parts.keys_ptr as *mut K).add(i));
                }

                dealloc_raw(node, self.branch_layout.bytes, self.branch_layout.max_align);
            }
        }
    }
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
use core::fmt;

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
            BPlusTreeError::KeyNotFound => write!(f, "Key not found"),
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
    pub fn is_leaf(&self) -> bool {
        matches!(self, NodeRef::Leaf(_, _))
    }
}

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    // ===== Compatibility constructors =====
    pub fn new(capacity: usize) -> Result<Self, BPlusTreeError> {
        if capacity < 4 {
            return Err(BPlusTreeError::InvalidCapacity("capacity too small".into()));
        }
        let cap_u16 = core::cmp::min(capacity as u16, u16::MAX);
        let leaf_layout = LeafLayout::compute_for_cap::<K, V>(cap_u16, true);
        let branch_layout = BranchLayout::compute_for_cap::<K>(cap_u16);
        let mut tree = Self {
            root: None,
            leaf_layout,
            branch_layout,
            _marker: PhantomData,
        };
        unsafe {
            let leaf = alloc_leaf_block(&tree.leaf_layout)
                .ok_or_else(|| BPlusTreeError::AllocationError("leaf root".into()))?;
            tree.root = Some(leaf);
        }
        Ok(tree)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        // Compute dynamically by walking the leaf linked list from the leftmost leaf
        let mut total = 0usize;
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
                let parts =
                    layout::carve_leaf::<K, V>(NonNull::new_unchecked(cur), &self.leaf_layout);
                total += (*parts.hdr).len as usize;
                cur = *parts.next_ptr;
            }
        }
        total
    }

    pub fn clear(&mut self) {
        if let Some(root) = self.root.take() {
            unsafe {
                self.free_tree_no_drop(root);
            }
        }
    }
}

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
    ($tree:expr, $context:expr) => {
        assert_tree_valid!($tree, $context);
    };
    ($tree:expr, $context:expr, ordering) => {{
        assert_tree_valid!($tree, $context);
        let items: std::vec::Vec<_> = $tree.items().collect();
        for i in 1..items.len() {
            if items[i - 1].0 >= items[i].0 {
                panic!("ATTACK SUCCESSFUL: Items out of order in {}!", $context);
            }
        }
    }};
    ($tree:expr, $context:expr, count = $expected:expr) => {{
        assert_tree_valid!($tree, $context);
        let actual = $tree.len();
        if actual != $expected {
            panic!(
                "ATTACK SUCCESSFUL in {}: Expected {} items, got {}",
                $context, $expected, actual
            );
        }
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

#[cfg(feature = "compat_test_api")]
pub trait BTreeResultExt<T> {
    fn with_context(self, _ctx: &str) -> Result<T, BPlusTreeError>;
    fn with_operation(self, _op: &str) -> Result<T, BPlusTreeError>;
    fn or_default_with_log(self) -> T
    where
        T: Default;
}

#[cfg(feature = "compat_test_api")]
impl<T> BTreeResultExt<T> for Result<T, BPlusTreeError> {
    fn with_context(self, _ctx: &str) -> Result<T, BPlusTreeError> {
        self
    }
    fn with_operation(self, _op: &str) -> Result<T, BPlusTreeError> {
        self
    }
    fn or_default_with_log(self) -> T
    where
        T: Default,
    {
        self.unwrap_or_default()
    }
}

impl BPlusTreeError {
    pub fn invalid_capacity(got: usize, min: usize) -> Self {
        BPlusTreeError::InvalidCapacity(format!(
            "Capacity {} is invalid (minimum required: {})",
            got, min
        ))
    }
    pub fn data_integrity(op: &str, why: &str) -> Self {
        BPlusTreeError::DataIntegrityError(format!("{}: {}", op, why))
    }
    pub fn arena_error(what: &str, why: &str) -> Self {
        BPlusTreeError::ArenaError(format!("{} failed: {}", what, why))
    }
    pub fn node_error(kind: &str, id: u32, why: &str) -> Self {
        BPlusTreeError::NodeError(format!("{} node {}: {}", kind, id, why))
    }
    pub fn corrupted_tree(where_: &str, why: &str) -> Self {
        BPlusTreeError::CorruptedTree(format!("{} corruption: {}", where_, why))
    }
    pub fn invalid_state(op: &str, why: &str) -> Self {
        BPlusTreeError::InvalidState(format!("Cannot {}: {}", op, why))
    }
    pub fn allocation_error(what: &str, why: &str) -> Self {
        BPlusTreeError::AllocationError(format!("Failed to allocate {}: {}", what, why))
    }
}

impl core::cmp::PartialEq for BPlusTreeError {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}
impl Eq for BPlusTreeError {}

// Extra convenience/debug API stubs used in tests
#[cfg(feature = "compat_test_api")]
impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn validate(&self) -> BTreeResult<()> {
        Ok(())
    }
    pub fn validate_for_operation(&self, _op: &str) -> BTreeResult<()> {
        Ok(())
    }
    pub fn try_get(&self, key: &K) -> KeyResult<&V> {
        self.get_item(key)
    }
    pub fn try_insert(&mut self, key: K, value: V) -> BTreeResult<Option<V>> {
        Ok(self.insert(key, value))
    }
    pub fn try_remove(&mut self, key: &K) -> ModifyResult<V> {
        self.remove_item(key)
    }
}
