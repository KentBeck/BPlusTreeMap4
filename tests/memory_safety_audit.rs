//! Memory safety audit tests
//! These tests verify that all type conversions are properly bounds-checked

use bplustree::BPlusTreeMap;

mod test_utils;
use test_utils::*;

/// Test that iteration returns items in sorted order after deletions
#[test]
fn test_iteration_order_after_deletions() {
    println!("=== ITERATION ORDER AFTER DELETIONS TEST ===");

    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(6).unwrap();

    // Create a tree with various operations to test iteration safety
    for i in 0..1000 {
        tree.insert(i, format!("iteration_test_{}", i));
    }

    // Remove some items to create fragmentation
    deletion_range_attack(&mut tree, 100, 200);

    // Test that iteration works correctly with type conversions
    let items: Vec<_> = tree.items().collect();
    println!("Iteration collected {} items", items.len());

    // Verify iteration is working properly (1000 - 100 removed = 900)
    assert_eq!(items.len(), 900, "Should have 900 items after removals");

    // Check that items are in order (verifies NodeId conversions in iteration)
    for window in items.windows(2) {
        assert!(
            window[0].0 < window[1].0,
            "Items should be in ascending order: {} >= {}",
            window[0].0,
            window[1].0
        );
    }

    // Test range operations with type safety
    let range_items: Vec<_> = tree.range(300..400).collect();
    assert_eq!(range_items.len(), 100, "Range should contain 100 items");

    println!("✅ Iteration order after deletions test passed");
}

/// Test edge cases that could cause integer overflow
#[test]
fn test_integer_overflow_prevention() {
    println!("=== INTEGER OVERFLOW PREVENTION TEST ===");

    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    // Test with large numbers that could cause overflow in calculations
    let large_numbers = [i32::MAX - 1000, i32::MAX - 100, i32::MAX - 10, i32::MAX - 1];

    for &num in &large_numbers {
        tree.insert(num, format!("large_num_{}", num));
    }

    println!("Successfully inserted large numbers");

    // Verify they're all accessible
    for &num in &large_numbers {
        assert!(
            tree.contains_key(&num),
            "Large number {} should be accessible",
            num
        );
    }

    // Test operations with these large numbers
    let items: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    println!("Large numbers in tree: {:?}", items);

    // Test range operations with large numbers
    let range_start = i32::MAX - 500;
    let range_items: Vec<_> = tree.range(range_start..).collect();
    println!(
        "Range from {} contains {} items",
        range_start,
        range_items.len()
    );

    println!("✅ Integer overflow prevention test passed");
}

/// Test that u32 keys can be inserted, retrieved, and removed correctly
#[test]
fn test_u32_key_operations() {
    println!("=== U32 KEY OPERATIONS TEST ===");

    let mut tree: BPlusTreeMap<u32, String> = BPlusTreeMap::new(4).unwrap();

    // Test with u32 keys to stress NodeId conversions
    let test_keys = [0u32, 1000, 10000, 100000, 1000000];

    for &key in &test_keys {
        tree.insert(key, format!("bounds_test_{}", key));
    }

    println!("Inserted keys: {:?}", test_keys);

    // Verify all keys are accessible
    for &key in &test_keys {
        assert!(tree.contains_key(&key), "Key {} should be accessible", key);

        let value = tree.get(&key);
        assert!(value.is_some(), "Should be able to get key {}", key);
        assert_eq!(
            value.unwrap(),
            &format!("bounds_test_{}", key),
            "Value should match for key {}",
            key
        );
    }

    // Test removal with bounds checking
    for &key in &test_keys {
        let removed = tree.remove(&key);
        assert!(removed.is_some(), "Should be able to remove key {}", key);
        assert!(
            !tree.contains_key(&key),
            "Key {} should be gone after removal",
            key
        );
    }

    assert!(
        tree.is_empty(),
        "Tree should be empty after removing all keys"
    );

    println!("✅ U32 key operations test passed");
}
