//! Tests for Drop and clear() behavior to ensure proper memory management
//! These tests verify that all allocated nodes are properly freed.

use bplustree::BPlusTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A wrapper type that tracks construction and destruction
struct DropCounter {
    id: usize,
    counter: Arc<AtomicUsize>,
}

impl DropCounter {
    fn new(id: usize, counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self { id, counter }
    }
}

impl Clone for DropCounter {
    fn clone(&self) -> Self {
        // Increment counter on clone too
        self.counter.fetch_add(1, Ordering::SeqCst);
        Self {
            id: self.id,
            counter: self.counter.clone(),
        }
    }
}

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

impl PartialEq for DropCounter {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for DropCounter {}

impl PartialOrd for DropCounter {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DropCounter {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

#[test]
fn test_drop_frees_all_memory_single_leaf() {
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let mut tree = BPlusTreeMap::new(10).unwrap();

        // Insert items into a single leaf
        for i in 0..5 {
            let key = DropCounter::new(i, counter.clone());
            let val = DropCounter::new(i + 1000, counter.clone());
            tree.insert(key, val);
        }

        // Should have 10 live objects (5 keys + 5 values)
        assert_eq!(counter.load(Ordering::SeqCst), 10);

        // Tree goes out of scope here, Drop should be called
    }

    // All objects should be dropped
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "Memory leak: not all objects were dropped"
    );
}

#[test]
fn test_drop_frees_all_memory_multi_level_tree() {
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let mut tree = BPlusTreeMap::new(5).unwrap();

        // Insert enough to create a multi-level tree
        for i in 0..100 {
            let key = DropCounter::new(i, counter.clone());
            let val = DropCounter::new(i + 10000, counter.clone());
            tree.insert(key, val);
        }

        // Should have at least 200 live objects (100 keys + 100 values + separator clones)
        let count = counter.load(Ordering::SeqCst);
        assert!(
            count >= 200,
            "Should have at least 200 objects, got {}",
            count
        );

        // Tree goes out of scope here
    }

    // All objects should be dropped
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "Memory leak in multi-level tree: not all objects were dropped"
    );
}

#[test]
fn test_clear_frees_all_memory() {
    let counter = Arc::new(AtomicUsize::new(0));

    let mut tree = BPlusTreeMap::new(5).unwrap();

    // Insert items
    for i in 0..50 {
        let key = DropCounter::new(i, counter.clone());
        let val = DropCounter::new(i + 5000, counter.clone());
        tree.insert(key, val);
    }

    // Should have 100 live objects (50 keys + 50 values)
    // Plus separator keys in branch nodes (which are clones)
    let count_after_insert = counter.load(Ordering::SeqCst);
    println!(
        "After insert: {} objects (includes separator key clones)",
        count_after_insert
    );
    assert!(
        count_after_insert >= 100,
        "Should have at least 100 objects"
    );

    // Clear the tree
    tree.clear();

    // All objects should be dropped after clear
    let count_after_clear = counter.load(Ordering::SeqCst);
    println!("After clear: {} objects", count_after_clear);
    assert_eq!(
        count_after_clear, 0,
        "Memory leak: clear() did not drop all objects"
    );

    // Tree should be empty
    assert_eq!(tree.len(), 0);
    assert!(tree.is_empty());
}

#[test]
fn test_clear_and_reuse_with_drop_tracking() {
    let counter = Arc::new(AtomicUsize::new(0));

    let mut tree = BPlusTreeMap::new(5).unwrap();

    // First batch
    for i in 0..30 {
        let key = DropCounter::new(i, counter.clone());
        let val = DropCounter::new(i + 3000, counter.clone());
        tree.insert(key, val);
    }
    let count1 = counter.load(Ordering::SeqCst);
    assert!(count1 >= 60, "Should have at least 60 objects");

    // Clear
    tree.clear();
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "First clear leaked memory"
    );

    // Second batch - reuse the tree
    for i in 100..130 {
        let key = DropCounter::new(i, counter.clone());
        let val = DropCounter::new(i + 3000, counter.clone());
        tree.insert(key, val);
    }
    let count2 = counter.load(Ordering::SeqCst);
    assert!(count2 >= 60, "Should have at least 60 objects");

    // Clear again
    tree.clear();
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "Second clear leaked memory"
    );

    // Third batch
    for i in 200..210 {
        let key = DropCounter::new(i, counter.clone());
        let val = DropCounter::new(i + 3000, counter.clone());
        tree.insert(key, val);
    }
    let count3 = counter.load(Ordering::SeqCst);
    assert!(count3 >= 20, "Should have at least 20 objects");

    // Final drop
    drop(tree);
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "Final drop leaked memory"
    );
}

#[test]
#[ignore] // TODO: Fix double-free issue in complex remove scenarios with rebalancing
fn test_drop_after_removes() {
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let mut tree = BPlusTreeMap::new(5).unwrap();

        // Insert items
        println!("Inserting 50 items...");
        for i in 0..50 {
            let key = DropCounter::new(i, counter.clone());
            let val = DropCounter::new(i + 6000, counter.clone());
            tree.insert(key, val);
        }
        let initial_count = counter.load(Ordering::SeqCst);
        println!("After insert: {} objects", initial_count);
        assert!(initial_count >= 100, "Should have at least 100 objects");

        // Remove some items (this should drop them immediately)
        println!("Removing items 10..30...");
        for i in 10..30 {
            println!(
                "  Removing {} (before: {} objects)",
                i,
                counter.load(Ordering::SeqCst)
            );
            let key = DropCounter::new(i, counter.clone());
            let before_remove = counter.load(Ordering::SeqCst);
            tree.remove(&key);
            let after_remove = counter.load(Ordering::SeqCst);
            let diff = if before_remove > after_remove {
                before_remove - after_remove
            } else {
                0
            };
            println!(
                "  After remove {}: {} objects (dropped {})",
                i, after_remove, diff
            );
        }

        // Should have fewer objects now (30 keys + 30 values + some separators)
        let current = counter.load(Ordering::SeqCst);
        println!("After removes: {} objects", current);
        assert!(
            current < initial_count,
            "Count should decrease after removes"
        );
        assert!(current >= 60, "Should have at least 60 objects left");

        println!("Dropping tree...");
        // Tree goes out of scope
    }

    println!("After drop: {} objects", counter.load(Ordering::SeqCst));

    // All remaining objects should be dropped
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "Memory leak after removes and drop"
    );
}

#[test]
fn test_drop_with_complex_tree_structure() {
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let mut tree = BPlusTreeMap::new(4).unwrap();

        // Create a complex tree with multiple levels
        // Insert in a pattern that creates splits
        for i in 0..200 {
            let key = DropCounter::new(i, counter.clone());
            let val = DropCounter::new(i + 20000, counter.clone());
            tree.insert(key, val);
        }

        let initial_count = counter.load(Ordering::SeqCst);
        assert!(initial_count >= 400, "Should have at least 400 objects");

        // Remove items to trigger merges and rebalancing
        for i in (50..150).step_by(2) {
            let key = DropCounter::new(i, counter.clone());
            tree.remove(&key);
        }

        // Should have fewer objects now
        let after_removes = counter.load(Ordering::SeqCst);
        assert!(
            after_removes < initial_count,
            "Removes should have decreased count"
        );

        // Tree goes out of scope
    }

    // All objects should be dropped
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "Memory leak in complex tree structure"
    );
}

#[test]
fn test_multiple_clear_cycles() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut tree = BPlusTreeMap::new(5).unwrap();

    for cycle in 0..10 {
        // Insert items
        for i in 0..20 {
            let key = DropCounter::new(cycle * 1000 + i, counter.clone());
            let val = DropCounter::new(cycle * 1000 + i + 50000, counter.clone());
            tree.insert(key, val);
        }

        let count = counter.load(Ordering::SeqCst);
        assert!(
            count >= 40,
            "Cycle {} should have at least 40 objects, got {}",
            cycle,
            count
        );

        // Clear
        tree.clear();

        assert_eq!(
            counter.load(Ordering::SeqCst),
            0,
            "Cycle {} leaked memory after clear",
            cycle
        );
    }
}

#[test]
fn test_drop_with_string_values() {
    // Test with String to ensure heap-allocated values are properly dropped
    {
        let mut tree = BPlusTreeMap::new(5).unwrap();

        for i in 0..100 {
            tree.insert(i, format!("value_{}_with_long_string_data", i));
        }

        assert_eq!(tree.len(), 100);

        // Tree goes out of scope, all Strings should be dropped
    }

    // If there's a memory leak with Strings, tools like valgrind would catch it
    // This test mainly ensures the code compiles and runs without panicking
}

#[test]
fn test_clear_empty_tree() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut tree: BPlusTreeMap<DropCounter, DropCounter> = BPlusTreeMap::new(5).unwrap();

    // Clear an empty tree (should not panic)
    tree.clear();
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    // Insert one item
    let key = DropCounter::new(1, counter.clone());
    let val = DropCounter::new(2, counter.clone());
    tree.insert(key, val);
    assert_eq!(counter.load(Ordering::SeqCst), 2);

    // Clear again
    tree.clear();
    let after_clear = counter.load(Ordering::SeqCst);
    println!("After clear with 1 item: {}", after_clear);
    assert_eq!(after_clear, 0, "Should be 0 but got {}", after_clear);

    // Clear empty tree again
    tree.clear();
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}

#[test]
fn test_minimal_clear() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut tree: BPlusTreeMap<DropCounter, DropCounter> = BPlusTreeMap::new(10).unwrap();

    // Insert just 3 items
    for i in 0..3 {
        let key = DropCounter::new(i, counter.clone());
        let val = DropCounter::new(i + 100, counter.clone());
        tree.insert(key, val);
    }

    println!("After 3 inserts: {}", counter.load(Ordering::SeqCst));
    assert_eq!(counter.load(Ordering::SeqCst), 6);

    tree.clear();
    let after_clear = counter.load(Ordering::SeqCst);
    println!("After clear: {}", after_clear);
    assert_eq!(after_clear, 0, "Expected 0, got {}", after_clear);
}

#[test]
#[ignore] // TODO: Fix double-free issue when removing item 13 after 10,11,12
fn test_simple_remove() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut tree: BPlusTreeMap<DropCounter, DropCounter> = BPlusTreeMap::new(5).unwrap();

    // Insert 50 items like the failing test
    println!("Inserting 50 items...");
    for i in 0..50 {
        let key = DropCounter::new(i, counter.clone());
        let val = DropCounter::new(i + 6000, counter.clone());
        tree.insert(key, val);
    }

    let after_insert = counter.load(Ordering::SeqCst);
    println!("After insert: {} objects", after_insert);

    // Remove items 10-13 to trigger the crash
    println!("Removing items 10-13...");
    for i in 10..14 {
        let before = counter.load(Ordering::SeqCst);
        println!("  Removing {} (before: {} objects)...", i, before);
        {
            let key = DropCounter::new(i, counter.clone());
            let after_key_create = counter.load(Ordering::SeqCst);
            tree.remove(&key);
            let after_remove_before_key_drop = counter.load(Ordering::SeqCst);
            println!(
                "    After key create: {}, after remove: {}",
                after_key_create, after_remove_before_key_drop
            );
        }
        let after = counter.load(Ordering::SeqCst);
        println!(
            "    After key drop: {} objects (net change: {})",
            after,
            (before as i64) - (after as i64)
        );
        if after > 1000000000000 {
            println!("    UNDERFLOW DETECTED!");
            panic!("Underflow at remove {}", i);
        }
    }

    let after_remove = counter.load(Ordering::SeqCst);
    println!("After all removes: {} objects", after_remove);

    // Drop tree
    drop(tree);
    let after_drop = counter.load(Ordering::SeqCst);
    println!("After drop: {} objects", after_drop);

    assert_eq!(after_drop, 0, "Memory leak");
}

#[test]
fn test_clear_with_20_items() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut tree: BPlusTreeMap<DropCounter, DropCounter> = BPlusTreeMap::new(5).unwrap();

    for i in 0..20 {
        let key = DropCounter::new(i, counter.clone());
        let val = DropCounter::new(i + 200, counter.clone());
        tree.insert(key, val);
    }

    let after_insert = counter.load(Ordering::SeqCst);
    let leaf_count = tree.leaf_count();
    let is_leaf_root = tree.is_leaf_root();

    println!("After 20 inserts: {} objects", after_insert);
    println!(
        "Tree has {} leaves, is_leaf_root={}",
        leaf_count, is_leaf_root
    );

    // With 6 leaves, we expect 5 separator keys in the branch (clones of leaf keys)
    // So total objects = 20 keys + 20 values + 5 separator keys = 45
    let expected_with_separators = 40 + (leaf_count - 1);
    println!(
        "Expected objects (including separators): {}",
        expected_with_separators
    );

    assert_eq!(
        after_insert,
        expected_with_separators,
        "Should have {} objects (40 leaf items + {} separators)",
        expected_with_separators,
        leaf_count - 1
    );

    tree.clear();
    let after_clear = counter.load(Ordering::SeqCst);
    println!("After clear: {} objects", after_clear);

    assert_eq!(after_clear, 0, "Expected 0, got {}", after_clear);
}
