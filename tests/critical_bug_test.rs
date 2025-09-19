/// Test to verify linked list integrity during merge operations
/// These tests ensure proper linked list maintenance during deletions
use bplustree::BPlusTreeMap;

mod test_utils;
use test_utils::*;

#[test]
fn test_linked_list_corruption_causes_data_loss() {
    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    // Create a specific pattern to test merge operations
    // This scenario triggers merge_with_left_leaf operations

    // Insert keys that will create multiple leaves
    let keys = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    for &key in &keys {
        tree.insert(key, format!("value_{}", key));
    }

    println!("Initial tree state:");
    println!("Leaf count: {}", tree.leaf_count());
    println!(
        "Items: {:?}",
        tree.items().map(|(k, _)| *k).collect::<Vec<_>>()
    );

    // Now delete items in a pattern that will trigger merging
    // This should cause the left leaf's next pointer to be incorrectly overwritten
    tree.remove(&40);
    tree.remove(&50);
    tree.remove(&60);

    println!("After deletions:");
    println!(
        "Items: {:?}",
        tree.items().map(|(k, _)| *k).collect::<Vec<_>>()
    );

    // Verify linked list integrity during merge operations

    // Check if all remaining items are still accessible
    let expected_remaining = vec![10, 20, 30, 70, 80, 90, 100];
    let actual_via_iteration: Vec<_> = tree.items().map(|(k, _)| *k).collect();

    // Check each item individually via get()
    for &key in &expected_remaining {
        if !tree.contains_key(&key) {
            panic!("Key {} became unreachable", key);
        }
    }

    // Check iteration consistency
    if actual_via_iteration != expected_remaining {
        panic!(
            "Linked list iteration error - expected {:?}, got {:?}",
            expected_remaining, actual_via_iteration
        );
    }

    // Test passed - linked list integrity maintained
    println!("Test passed - linked list integrity verified");
}

#[test]
fn demonstrate_memory_leak_accumulation() {
    println!("\n=== DEMONSTRATING MEMORY LEAK ACCUMULATION ===");

    // This test shows how the memory leak accumulates with multiple root splits
    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    for round in 1..=5 {
        // Add enough items to force root splits
        let start = (round - 1) * 10;
        for i in start..start + 10 {
            tree.insert(i, format!("value_{}", i));
        }

        let allocated = tree.allocated_leaf_count();
        let in_tree = tree.leaf_count();
        let leaked = allocated - in_tree;

        println!(
            "Round {}: {} allocated, {} in tree, {} leaked",
            round, allocated, in_tree, leaked
        );

        // The bug causes the leak to grow with each root split
        if leaked > 0 {
            println!("  ✗ Memory leak detected: {} nodes", leaked);
        }
    }
}

#[test]
fn test_invariants_after_problematic_operations() {
    println!("\n=== TESTING INVARIANTS AFTER PROBLEMATIC OPERATIONS ===");

    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(5).unwrap(); // Odd capacity

    // Perform operations that might violate invariants due to the bugs
    insert_sequential_range(&mut tree, 20);

    println!("After insertions with odd capacity:");
    println!("  Invariants valid: {}", tree.check_invariants());
    println!("  Leaf sizes: {:?}", tree.leaf_sizes());

    // Delete items to trigger rebalancing/merging
    for i in 8..17 {
        tree.remove(&i);
    }

    println!("After deletions:");
    println!("  Invariants valid: {}", tree.check_invariants());
    println!("  Leaf sizes: {:?}", tree.leaf_sizes());

    // Check for specific invariant violations
    let _min_keys = 2; // Current incorrect calculation for capacity 5
    let correct_min_keys = 3; // What it should be

    let leaf_sizes = tree.leaf_sizes();
    let violations: Vec<_> = leaf_sizes
        .iter()
        .filter(|&&size| size > 0 && size < correct_min_keys)
        .collect();

    if !violations.is_empty() {
        println!(
            "  ✗ Invariant violations: {} leaves below correct minimum",
            violations.len()
        );
    }
}

#[test]
fn stress_test_arena_consistency() {
    println!("\n=== STRESS TESTING ARENA CONSISTENCY ===");

    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    // Perform many operations to stress test the arena
    for cycle in 0..10 {
        // Insert batch
        for i in 0..20 {
            tree.insert(cycle * 100 + i, format!("value_{}_{}", cycle, i));
        }

        // Delete some items
        for i in 5..15 {
            tree.remove(&(cycle * 100 + i));
        }

        let allocated_leaves = tree.allocated_leaf_count();
        let free_leaves = tree.free_leaf_count();
        let actual_leaves = tree.leaf_count();

        if cycle % 3 == 0 {
            println!(
                "Cycle {}: allocated={}, free={}, in_tree={}",
                cycle, allocated_leaves, free_leaves, actual_leaves
            );
        }

        // Check for accumulating inconsistencies
        if allocated_leaves > actual_leaves * 2 {
            println!("  ⚠ WARNING: Large discrepancy between allocated and used nodes");
        }
    }

    // Final consistency check
    let final_allocated = tree.allocated_leaf_count();
    let final_in_tree = tree.leaf_count();

    println!(
        "Final state: {} allocated, {} in tree",
        final_allocated, final_in_tree
    );

    if final_allocated > final_in_tree {
        println!(
            "  ✗ Final inconsistency: {} extra allocated nodes",
            final_allocated - final_in_tree
        );
    }
}
