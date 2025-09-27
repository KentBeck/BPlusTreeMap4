use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

mod test_utils;
use test_utils::*;

struct TrackingAllocator;

static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
            ALLOC_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !ptr.is_null() {
            DEALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
            DEALLOC_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        }
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn reset_alloc_metrics() {
    ALLOC_CALLS.store(0, Ordering::SeqCst);
    ALLOC_BYTES.store(0, Ordering::SeqCst);
    DEALLOC_CALLS.store(0, Ordering::SeqCst);
    DEALLOC_BYTES.store(0, Ordering::SeqCst);
}

fn alloc_metrics() -> (usize, usize, usize, usize) {
    (
        ALLOC_CALLS.load(Ordering::SeqCst),
        ALLOC_BYTES.load(Ordering::SeqCst),
        DEALLOC_CALLS.load(Ordering::SeqCst),
        DEALLOC_BYTES.load(Ordering::SeqCst),
    )
}

#[test]
fn test_basic_deletion() {
    let mut tree = create_tree_capacity_int(4);
    tree.insert(42, 420);
    assert_eq!(tree.get(&42), Some(&420));
    assert_eq!(tree.remove(&42), Some(420));
    assert_eq!(tree.get(&42), None);
}

#[test]
fn test_delete_nonexistent_key() {
    let mut tree = create_tree_capacity_int(4);
    tree.insert(42, 420);
    assert_eq!(tree.remove(&42), Some(420));
    assert_eq!(tree.remove(&42), None);
}

#[test]
fn test_delete_from_branch_tree() {
    let mut tree = create_tree_capacity_int(4);
    for i in 0..8 {
        tree.insert(i, i * 10);
    }
    assert!(!tree.is_leaf_root());
    assert_eq!(tree.remove(&3), Some(30));
    assert_eq!(tree.get(&3), None);
    for i in 0..8 {
        if i != 3 {
            assert_eq!(tree.get(&i), Some(&(i * 10)));
        }
    }
}

#[test]
fn test_delete_forces_root_collapse() {
    let mut tree = create_tree_capacity_int(4);
    for i in 0..5 {
        tree.insert(i, i * 10);
    }
    assert!(!tree.is_leaf_root());
    assert_eq!(
        tree.leaf_count(),
        2,
        "setup should create a branch root with two leaves"
    );

    reset_alloc_metrics();
    let removed = tree.remove(&0);
    let (alloc_calls, _alloc_bytes, dealloc_calls, dealloc_bytes) = alloc_metrics();

    assert_eq!(removed, Some(0));
    assert_eq!(
        alloc_calls, 0,
        "root collapse should not allocate new nodes"
    );
    assert_eq!(
        dealloc_calls, 2,
        "root collapse should free the emptied leaf and the old branch root",
    );

    let expected_dealloc_bytes = tree.leaf_layout().bytes + tree.branch_layout().bytes;
    assert_eq!(
        dealloc_bytes, expected_dealloc_bytes,
        "freed {} bytes but expected {}",
        dealloc_bytes, expected_dealloc_bytes,
    );

    assert_eq!(tree.get(&0), None);
    for i in 1..5 {
        assert_eq!(tree.get(&i), Some(&(i * 10)));
    }
    assert!(tree.is_leaf_root());
    assert_eq!(
        tree.leaf_count(),
        1,
        "root collapse should leave a single leaf"
    );
}

#[test]
fn test_delete_last_forces_root_collapse() {
    let mut tree = create_tree_capacity_int(4);
    for i in 0..5 {
        tree.insert(i, i * 10);
    }
    assert!(!tree.is_leaf_root());
    assert_eq!(
        tree.leaf_count(),
        2,
        "setup should create a branch root with two leaves"
    );

    reset_alloc_metrics();
    let removed = tree.remove(&4);
    let (alloc_calls, _alloc_bytes, dealloc_calls, dealloc_bytes) = alloc_metrics();

    assert_eq!(removed, Some(40));
    assert_eq!(
        alloc_calls, 0,
        "root collapse should not allocate new nodes"
    );
    assert_eq!(
        dealloc_calls, 2,
        "root collapse should free the emptied leaf and the old branch root",
    );

    let expected_dealloc_bytes = tree.leaf_layout().bytes + tree.branch_layout().bytes;
    assert_eq!(
        dealloc_bytes, expected_dealloc_bytes,
        "freed {} bytes but expected {}",
        dealloc_bytes, expected_dealloc_bytes,
    );

    for i in 0..4 {
        assert_eq!(tree.get(&i), Some(&(i * 10)));
    }
    assert_eq!(tree.get(&4), None);
    assert!(tree.is_leaf_root());
    assert_eq!(
        tree.leaf_count(),
        1,
        "root collapse should leave a single leaf"
    );
}
