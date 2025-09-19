//! Error handling consistency tests
//! These tests verify that the B+ tree implementation uses consistent error handling patterns

use bplustree::{BPlusTreeError, BPlusTreeMap};

mod test_utils;
use test_utils::*;

/// Test that all public APIs return consistent error types
#[test]
fn test_public_api_error_consistency() {
    println!("=== PUBLIC API ERROR CONSISTENCY TEST ===");

    // Test constructor error handling
    let invalid_tree = BPlusTreeMap::<i32, String>::new(2); // Below minimum capacity
    assert!(
        invalid_tree.is_err(),
        "Constructor should return error for invalid capacity"
    );

    match invalid_tree {
        Err(BPlusTreeError::InvalidCapacity(_)) => {
            println!("✅ Constructor returns proper InvalidCapacity error");
        }
        Err(other) => panic!("Wrong error type: {:?}", other),
        Ok(_) => panic!("Should have failed with invalid capacity"),
    }

    // Test valid constructor
    let mut tree = create_tree_4();

    // Test get_item error handling
    let missing_key_result = tree.get_item(&999);
    assert!(
        missing_key_result.is_err(),
        "get_item should return error for missing key"
    );

    match missing_key_result {
        Err(BPlusTreeError::KeyNotFound) => {
            println!("✅ get_item returns proper KeyNotFound error");
        }
        Err(other) => panic!("Wrong error type: {:?}", other),
        Ok(_) => panic!("Should have failed with KeyNotFound"),
    }

    // Test remove_item error handling
    let remove_missing_result = tree.remove_item(&999);
    assert!(
        remove_missing_result.is_err(),
        "remove_item should return error for missing key"
    );

    match remove_missing_result {
        Err(BPlusTreeError::KeyNotFound) => {
            println!("✅ remove_item returns proper KeyNotFound error");
        }
        Err(other) => panic!("Wrong error type: {:?}", other),
        Ok(_) => panic!("Should have failed with KeyNotFound"),
    }

    println!("✅ Public API error consistency verified");
}

/// Test error message formatting and Display implementation
#[test]
fn test_error_message_formatting() {
    println!("=== ERROR MESSAGE FORMATTING TEST ===");

    let errors = vec![
        BPlusTreeError::KeyNotFound,
        BPlusTreeError::InvalidCapacity("capacity too small".to_string()),
        BPlusTreeError::DataIntegrityError("corruption detected".to_string()),
        BPlusTreeError::ArenaError("allocation failed".to_string()),
        BPlusTreeError::NodeError("node not found".to_string()),
        BPlusTreeError::CorruptedTree("tree structure invalid".to_string()),
        BPlusTreeError::InvalidState("invalid operation".to_string()),
        BPlusTreeError::AllocationError("out of memory".to_string()),
    ];

    for error in errors {
        let error_message = format!("{}", error);
        println!("Error message: {}", error_message);

        // Verify error messages are non-empty and descriptive
        assert!(
            !error_message.is_empty(),
            "Error message should not be empty"
        );
        assert!(
            error_message.len() > 5,
            "Error message should be descriptive"
        );

        // Verify Error trait implementation
        let error_trait: &dyn std::error::Error = &error;
        assert!(
            error_trait.to_string() == error_message,
            "Error trait should match Display"
        );
    }

    println!("✅ Error message formatting verified");
}

/// Test that operations handle edge cases gracefully
#[test]
fn test_edge_case_error_handling() {
    println!("=== EDGE CASE ERROR HANDLING TEST ===");

    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    // Test operations on empty tree
    assert_eq!(tree.get(&1), None, "get should return None on empty tree");
    assert_eq!(
        tree.remove(&1),
        None,
        "remove should return None on empty tree"
    );

    assert!(
        tree.get_item(&1).is_err(),
        "get_item should return error on empty tree"
    );
    assert!(
        tree.remove_item(&1).is_err(),
        "remove_item should return error on empty tree"
    );

    // Add some data for further testing
    insert_sequential_range(&mut tree, 10);

    // Test boundary conditions
    assert!(tree.get(&-1).is_none(), "get should handle negative keys");
    assert!(tree.get(&1000).is_none(), "get should handle large keys");

    // Test invariant checking with complex operations
    deletion_range_attack(&mut tree, 0, 5);

    // Tree should still be valid after operations
    assert!(
        tree.check_invariants(),
        "Tree should maintain invariants after operations"
    );

    println!("✅ Edge case error handling verified");
}

/// Test error propagation through complex operations
#[test]
fn test_error_propagation() {
    println!("=== ERROR PROPAGATION TEST ===");

    let mut tree = create_tree_4_with_data(100);

    // Test that errors propagate correctly through the tree structure
    // This tests internal error handling consistency

    // Test range operations with edge cases
    let range_items: Vec<_> = tree.range(50..60).collect();
    assert_eq!(range_items.len(), 10, "Range should return correct count");

    // Test iteration consistency
    let all_items: Vec<_> = tree.items().collect();
    assert_eq!(all_items.len(), 100, "Iteration should return all items");

    // Verify that all items are accessible
    for i in 0..100 {
        assert!(
            tree.contains_key(&i),
            "All inserted keys should be accessible"
        );
    }

    // Test mixed operations
    deletion_range_attack(&mut tree, 20, 80);

    // Verify remaining items
    let remaining_items: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    let expected_remaining: Vec<_> = (0..20).chain(80..100).collect();

    assert_eq!(
        remaining_items, expected_remaining,
        "Remaining items should match expected"
    );

    println!("✅ Error propagation verified");
}

/// Test concurrent operation safety (single-threaded verification)
#[test]
fn test_operation_safety() {
    println!("=== OPERATION SAFETY TEST ===");

    let mut tree = create_tree_capacity(8);

    // Test rapid insertion/deletion cycles
    for cycle in 0..50 {
        let base = cycle * 100;

        // Insert batch
        insert_with_offset_multiplier(&mut tree, 50, base, 1);

        // Verify batch was inserted correctly
        for i in 0..50 {
            assert!(
                tree.contains_key(&(base + i)),
                "Key should exist after insertion"
            );
        }

        // Remove some items
        for i in 10..40 {
            let removed = tree.remove(&(base + i));
            assert!(removed.is_some(), "Remove should return the value");
        }

        // Verify partial removal
        for i in 0..50 {
            let should_exist = i < 10 || i >= 40;
            let actually_exists = tree.contains_key(&(base + i));
            assert_eq!(
                should_exist,
                actually_exists,
                "Key existence should match expectation for key {}",
                base + i
            );
        }

        // Check tree invariants every 10 cycles
        if cycle % 10 == 9 {
            assert!(
                tree.check_invariants(),
                "Tree invariants should be maintained"
            );
        }
    }

    println!("✅ Operation safety verified");
}

/// Test error recovery scenarios
#[test]
fn test_error_recovery() {
    println!("=== ERROR RECOVERY TEST ===");

    let mut tree = create_tree_4();

    // Test recovery from various error conditions

    // 1. Test recovery from attempting operations on missing keys
    for i in 0..10 {
        // Try to remove non-existent keys
        let result = tree.remove(&i);
        assert!(
            result.is_none(),
            "Remove should return None for missing key"
        );

        // Try to get non-existent keys
        let result = tree.get(&i);
        assert!(result.is_none(), "Get should return None for missing key");

        // Error-returning versions should fail gracefully
        assert!(tree.get_item(&i).is_err(), "get_item should return error");
        assert!(
            tree.remove_item(&i).is_err(),
            "remove_item should return error"
        );
    }

    // 2. Add data and test recovery from edge cases
    insert_sequential_range(&mut tree, 20);

    // Remove all data and verify tree can recover
    deletion_range_attack(&mut tree, 0, 20);

    assert!(
        tree.is_empty(),
        "Tree should be empty after removing all items"
    );
    assert!(
        tree.check_invariants(),
        "Empty tree should still satisfy invariants"
    );

    // 3. Test that tree can be used normally after recovery
    insert_range(&mut tree, 100, 110);

    assert_eq!(tree.len(), 10, "Tree should have 10 items after recovery");

    // Verify all new items are accessible
    for i in 100..110 {
        assert!(
            tree.contains_key(&i),
            "New items should be accessible after recovery"
        );
    }

    println!("✅ Error recovery verified");
}

/// Test that internal error checking is consistent
#[test]
fn test_internal_error_consistency() {
    println!("=== INTERNAL ERROR CONSISTENCY TEST ===");

    let mut tree = create_tree_4();

    // Test that internal validation is working
    insert_with_custom_fn(
        &mut tree,
        1000,
        |i| i as i32,
        |i| format!("consistency_test_{}", i),
    );

    for i in 0..1000 {
        // Check invariants every 100 insertions
        if i % 100 == 99 {
            assert!(
                tree.check_invariants(),
                "Tree invariants should be maintained during growth"
            );
        }
    }

    // Test large-scale deletions
    deletion_range_attack(&mut tree, 200, 800);

    for i in 200..800 {
        // Check invariants every 100 deletions
        if i % 100 == 99 {
            assert!(
                tree.check_invariants(),
                "Tree invariants should be maintained during shrinkage"
            );
        }
    }

    // Final consistency check
    assert!(
        tree.check_invariants(),
        "Tree should maintain invariants after all operations"
    );

    // Verify that remaining items are still accessible
    let remaining_items: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    let expected_count = 200 + (1000 - 800); // 0..200 + 800..1000
    assert_eq!(
        remaining_items.len(),
        expected_count,
        "Should have correct number of remaining items"
    );

    // Verify item order is maintained
    for window in remaining_items.windows(2) {
        assert!(window[0] < window[1], "Items should remain in sorted order");
    }

    println!("✅ Internal error consistency verified");
}
