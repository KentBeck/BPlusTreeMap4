//! Linked list integrity verification tests
//! These tests verify proper linked list maintenance during merge operations

mod test_utils;
use test_utils::*;

/// INTENSIVE TEST: Verify linked list integrity through aggressive merge patterns
#[test]
fn test_intensive_linked_list_corruption_detection() {
    println!("=== INTENSIVE LINKED LIST INTEGRITY VERIFICATION ===");

    let mut tree = create_tree_4();

    // Phase 1: Create a complex tree structure with multiple leaves
    println!("\n--- Phase 1: Building complex tree structure ---");
    let initial_keys: Vec<i32> = (0..100).step_by(10).collect(); // [0, 10, 20, ..., 90]

    for &key in &initial_keys {
        tree.insert(key, format!("value_{}", key));
    }

    let initial_items: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    println!("Initial tree items: {:?}", initial_items);
    println!("Initial leaf count: {}", tree.leaf_count());

    // Phase 2: Strategic deletions to force merges
    println!("\n--- Phase 2: Strategic deletions to trigger merges ---");

    // Remove middle elements to create underfull nodes that need merging
    let keys_to_remove = vec![20, 30, 40, 50, 60, 70];
    for &key in &keys_to_remove {
        println!("Removing key: {}", key);
        tree.remove(&key);

        // Verify linked list consistency after each removal
        let items_after_removal: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        println!("  Items after removal: {:?}", items_after_removal);

        // Verify all remaining items are accessible via get()
        for &item_key in &items_after_removal {
            if !tree.contains_key(&item_key) {
                panic!(
                    "INTEGRITY ERROR: Key {} not accessible via get() but found in iteration",
                    item_key
                );
            }
        }

        // Verify no extra items exist that aren't in iteration
        for &original_key in &initial_keys {
            let should_exist = !keys_to_remove[..keys_to_remove
                .iter()
                .position(|&x| x == key)
                .unwrap_or(keys_to_remove.len())
                + 1]
                .contains(&original_key);
            let actually_exists = tree.contains_key(&original_key);

            if should_exist != actually_exists {
                if should_exist {
                    panic!(
                        "INTEGRITY ERROR: Key {} should exist but is not accessible",
                        original_key
                    );
                } else {
                    panic!(
                        "INTEGRITY ERROR: Key {} should not exist but is still accessible",
                        original_key
                    );
                }
            }
        }
    }

    let remaining_after_phase2: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    let expected_after_phase2 = vec![0, 10, 80, 90];

    if remaining_after_phase2 != expected_after_phase2 {
        panic!(
            "Phase 2 integrity error: expected {:?}, got {:?}",
            expected_after_phase2, remaining_after_phase2
        );
    }

    println!("✅ Phase 2 completed: {}", tree.leaf_count());

    // Phase 3: Rebuild and test alternating pattern
    println!("\n--- Phase 3: Rebuild and test alternating deletion ---");

    // Add back some elements to create a new pattern
    for i in 1..10 {
        tree.insert(i * 5, format!("rebuild_{}", i * 5));
    }

    let before_alternating: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    println!("Before alternating deletions: {:?}", before_alternating);

    // Remove every other element to stress the linked list
    let keys_to_remove_alternating: Vec<_> = before_alternating
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, &k)| k)
        .collect();

    for &key in &keys_to_remove_alternating {
        tree.remove(&key);
    }

    let after_alternating: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    println!("After alternating deletions: {:?}", after_alternating);

    // Verify alternating pattern worked correctly
    let expected_alternating: Vec<_> = before_alternating
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, &k)| k)
        .collect();

    if after_alternating != expected_alternating {
        panic!(
            "Alternating deletion integrity error: expected {:?}, got {:?}",
            expected_alternating, after_alternating
        );
    }

    println!("✅ Phase 3 completed: {}", tree.leaf_count());

    println!("\n✅ INTENSIVE LINKED LIST INTEGRITY TEST PASSED");
}

/// Test specific merge scenarios that could corrupt linked list pointers
#[test]
fn test_merge_scenarios_linked_list_integrity() {
    println!("=== MERGE SCENARIOS LINKED LIST INTEGRITY TEST ===");

    // Test 1: Left merge scenario
    {
        println!("\n--- Test 1: Left merge scenario ---");
        let mut tree = create_tree_4();

        // Create pattern: [A] -> [B] -> [C] -> [D]
        // Then merge B into A, should result in: [A+B] -> [C] -> [D]
        insert_sequential_range(&mut tree, 16);

        let before_merge: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        println!("Before deletions: {:?}", before_merge);

        // Delete elements to force left merge
        deletion_range_attack(&mut tree, 4, 8);

        let after_merge: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        println!("After deletions: {:?}", after_merge);

        // Verify no gaps in sequence
        let expected: Vec<_> = (0..4).chain(8..16).collect();
        if after_merge != expected {
            panic!(
                "Left merge integrity error: expected {:?}, got {:?}",
                expected, after_merge
            );
        }

        println!("✅ Left merge test passed");
    }

    // Test 2: Right merge scenario
    {
        println!("\n--- Test 2: Right merge scenario ---");
        let mut tree = create_tree_4();

        insert_sequential_range(&mut tree, 16);

        let before_merge: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        println!("Before deletions: {:?}", before_merge);

        // Delete elements to force right merge
        deletion_range_attack(&mut tree, 8, 12);

        let after_merge: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        println!("After deletions: {:?}", after_merge);

        // Verify no gaps in sequence
        let expected: Vec<_> = (0..8).chain(12..16).collect();
        if after_merge != expected {
            panic!(
                "Right merge integrity error: expected {:?}, got {:?}",
                expected, after_merge
            );
        }

        println!("✅ Right merge test passed");
    }

    // Test 3: Cascading merges
    {
        println!("\n--- Test 3: Cascading merges ---");
        let mut tree = create_tree_4_with_data(32);

        let before_cascade: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        println!("Before cascading deletions: {:?}", before_cascade);

        // Delete large ranges to force cascading merges
        deletion_range_attack(&mut tree, 8, 24);

        let after_cascade: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        println!("After cascading deletions: {:?}", after_cascade);

        // Verify no gaps in sequence
        let expected: Vec<_> = (0..8).chain(24..32).collect();
        if after_cascade != expected {
            panic!(
                "Cascading merge integrity error: expected {:?}, got {:?}",
                expected, after_cascade
            );
        }

        println!("✅ Cascading merge test passed");
    }

    println!("\n✅ ALL MERGE SCENARIOS PASSED");
}

/// Test edge cases in linked list management
#[test]
fn test_linked_list_edge_cases() {
    println!("=== LINKED LIST EDGE CASES TEST ===");

    // Edge case 1: Single leaf operations
    {
        let mut tree = create_tree_4();
        tree.insert(1, "single".to_string());

        let items: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        assert_eq!(items, vec![1], "Single leaf case failed");

        tree.remove(&1);
        let items_after: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        assert!(items_after.is_empty(), "Single leaf removal failed");

        println!("✅ Single leaf operations passed");
    }

    // Edge case 2: Two leaf operations
    {
        let mut tree = create_tree_4_with_data(8);

        // Should have exactly 2 leaves
        assert!(tree.leaf_count() >= 2, "Should have at least 2 leaves");

        // Remove elements from first leaf
        deletion_range_attack(&mut tree, 0, 3);

        let remaining: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        let expected: Vec<_> = (3..8).collect();
        assert_eq!(remaining, expected, "Two leaf partial removal failed");

        println!("✅ Two leaf operations passed");
    }

    // Edge case 3: Empty tree after operations
    {
        let mut tree = create_tree_4_with_data(10);
        deletion_range_attack(&mut tree, 0, 10);

        let final_items: Vec<_> = tree.items().map(|(k, _)| *k).collect();
        assert!(
            final_items.is_empty(),
            "Tree should be empty after removing all items"
        );

        println!("✅ Empty tree operations passed");
    }

    println!("\n✅ ALL EDGE CASES PASSED");
}

/// Stress test for linked list consistency under heavy operations
#[test]
fn test_linked_list_stress_consistency() {
    println!("=== LINKED LIST STRESS CONSISTENCY TEST ===");

    let mut tree = create_tree_6();

    for round in 0..10 {
        println!("\n--- Stress Round {} ---", round + 1);

        // Insert a batch of items
        let base = round * 100;
        for i in 0..50 {
            tree.insert(base + i, format!("stress_{}_{}", round, i));
        }

        // Remove some items in a pattern that could stress linked list
        for i in 10..40 {
            if i % 3 == 0 {
                tree.remove(&(base + i));
            }
        }

        // Verify linked list consistency
        let items: Vec<_> = tree.items().map(|(k, _)| *k).collect();

        // Check that items are in sorted order (linked list integrity)
        for window in items.windows(2) {
            if window[0] >= window[1] {
                panic!("Linked list order error: {} >= {}", window[0], window[1]);
            }
        }

        // Check that all items in iteration are accessible via get
        for &key in &items {
            if !tree.contains_key(&key) {
                panic!(
                    "Linked list integrity error: key {} in iteration but not accessible",
                    key
                );
            }
        }

        if round % 3 == 2 {
            println!(
                "  Round {}: {} items, linked list consistent ✓",
                round + 1,
                items.len()
            );
        }
    }

    println!("\n✅ STRESS TEST COMPLETED - LINKED LIST CONSISTENT");
}
