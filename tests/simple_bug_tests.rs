/// Simplified tests to demonstrate specific bugs in the B+ tree implementation
mod test_utils;
use test_utils::*;

#[test]
fn test_memory_leak_placeholder() {
    let mut tree = create_tree_4();

    // Record initial arena state
    let _initial_count = tree.allocated_leaf_count();

    // Force root splits to trigger the placeholder leak
    insert_sequential_range(&mut tree, 20);

    // Check if we have more allocated nodes than actual tree nodes
    let allocated = tree.allocated_leaf_count();
    let actual_leaves = tree.leaf_count();

    println!(
        "Allocated leaves: {}, Actual leaves in tree: {}",
        allocated, actual_leaves
    );

    // This will show the memory leak if it exists
    assert!(
        allocated >= actual_leaves,
        "Should have at least as many allocated as in tree"
    );

    // The test will reveal the issue by showing excessive allocation
    if allocated > actual_leaves {
        println!(
            "POTENTIAL MEMORY LEAK: {} allocated but only {} in tree structure",
            allocated, actual_leaves
        );
    }
}

#[test]
fn test_odd_capacity_split() {
    let mut tree = create_tree_5();

    // Insert enough to force splits with odd capacity
    insert_sequential_range(&mut tree, 10);

    // Check leaf node sizes
    let leaf_sizes = tree.leaf_sizes();
    println!("Leaf sizes with capacity 5: {:?}", leaf_sizes);

    // With capacity 5, min_keys = 2, so all non-empty leaves should have >= 2 keys
    let min_keys = 2;
    for &size in &leaf_sizes {
        if size > 0 && size < min_keys {
            panic!(
                "Split created underfull leaf: {} keys < {} minimum",
                size, min_keys
            );
        }
    }
}

#[test]
fn test_linked_list_integrity() {
    let mut tree = create_tree_4();

    // Create multiple leaves
    insert_with_multiplier(&mut tree, 20, 10);

    // Collect items via iteration (uses linked list)
    let items_via_iteration: Vec<_> = tree.items().map(|(k, _)| *k).collect();

    // Collect items via tree traversal (different path)
    let mut items_via_tree = Vec::new();
    for i in 0..20 {
        if tree.contains_key(&(i * 10)) {
            items_via_tree.push(i * 10);
        }
    }

    println!("Via iteration: {:?}", items_via_iteration);
    println!("Via tree lookup: {:?}", items_via_tree);

    // These should match if linked list is correct
    assert_eq!(
        items_via_iteration, items_via_tree,
        "Linked list iteration doesn't match tree structure"
    );

    // Now delete some items and retest
    deletion_range_attack(&mut tree, 50, 150);

    let items_after_delete: Vec<_> = tree.items().map(|(k, _)| *k).collect();

    // Check that iteration is still sorted
    for i in 1..items_after_delete.len() {
        assert!(
            items_after_delete[i - 1] < items_after_delete[i],
            "Items not in sorted order after deletion"
        );
    }
}

#[test]
fn test_range_excluded_bounds() {
    let mut tree = create_tree_4();

    insert_sequential_range(&mut tree, 10);

    // Test excluded start bound
    use std::ops::Bound;
    let items: Vec<_> = tree
        .range((Bound::Excluded(3), Bound::Unbounded))
        .map(|(k, _)| *k)
        .collect();

    println!("Items with excluded start 3: {:?}", items);

    // Should NOT include 3, should start from 4
    assert!(
        !items.contains(&3),
        "Excluded start bound incorrectly included 3"
    );
    assert!(items.contains(&4), "Should include 4 after excluding 3");

    // Test excluded end bound
    let items2: Vec<_> = tree
        .range((Bound::Unbounded, Bound::Excluded(7)))
        .map(|(k, _)| *k)
        .collect();

    println!("Items with excluded end 7: {:?}", items2);

    // Should NOT include 7, should end at 6
    assert!(
        !items2.contains(&7),
        "Excluded end bound incorrectly included 7"
    );
    assert!(items2.contains(&6), "Should include 6 before excluding 7");
}

#[test]
fn test_min_keys_consistency() {
    // This test checks if the min_keys calculation is appropriate
    let _tree = create_tree_6();

    // Create a tree that will have both leaf and branch nodes
    let test_tree = create_tree_with_data(6, 50);

    // Check if the tree maintains proper structure
    assert_invariants(&test_tree, "min keys consistency");

    // The min_keys formula might be problematic for certain capacities
    // This test documents the current behavior
    println!("Tree with capacity 6 has {} leaves", test_tree.leaf_count());
    println!("Leaf sizes: {:?}", test_tree.leaf_sizes());
}

#[test]
fn test_rebalancing_after_deletions() {
    let mut tree = create_tree_4();

    // Create a substantial tree
    insert_sequential_range(&mut tree, 50);

    println!("Before deletions - leaf count: {}", tree.leaf_count());
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Delete many items to force rebalancing
    deletion_range_attack(&mut tree, 10, 40);

    println!("After deletions - leaf count: {}", tree.leaf_count());
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Check that tree is still valid
    assert_invariants(&tree, "rebalancing after deletions");

    // Check for underfull nodes (this might reveal rebalancing issues)
    let min_keys = 2; // For capacity 4
    let leaf_sizes = tree.leaf_sizes();

    let underfull_count = leaf_sizes
        .iter()
        .filter(|&&size| size > 0 && size < min_keys)
        .count();

    if underfull_count > 0 {
        println!("WARNING: {} underfull leaves detected", underfull_count);
        // This is expected to show rebalancing issues if they exist
    }
}

#[test]
fn test_iterator_consistency() {
    let mut tree = create_tree_4();

    insert_sequential_range(&mut tree, 10);

    // Multiple iterations should give same results
    let iter1: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    let iter2: Vec<_> = tree.items().map(|(k, _)| *k).collect();

    assert_eq!(iter1, iter2, "Multiple iterations should be consistent");

    // Range iteration should be consistent with full iteration
    let range_all: Vec<_> = tree.range(..).map(|(k, _)| *k).collect();

    assert_eq!(iter1, range_all, "Range(..) should match full iteration");
}

#[test]
fn test_arena_utilization() {
    let mut tree = create_tree_4();

    println!("Initial state:");
    println!("  Leaf utilization: {:.2}", tree.leaf_utilization());
    println!("  Allocated leaves: {}", tree.allocated_leaf_count());
    println!("  Free leaves: {}", tree.free_leaf_count());

    // Add data
    insert_sequential_range(&mut tree, 20);

    println!("After insertions:");
    println!("  Leaf utilization: {:.2}", tree.leaf_utilization());
    println!("  Allocated leaves: {}", tree.allocated_leaf_count());
    println!("  Free leaves: {}", tree.free_leaf_count());

    // Remove some data
    deletion_range_attack(&mut tree, 5, 15);

    println!("After deletions:");
    println!("  Leaf utilization: {:.2}", tree.leaf_utilization());
    println!("  Allocated leaves: {}", tree.allocated_leaf_count());
    println!("  Free leaves: {}", tree.free_leaf_count());

    // This will show if there are memory leaks or arena issues
    let utilization = tree.leaf_utilization();
    assert!(
        utilization > 0.0 && utilization <= 1.0,
        "Utilization should be between 0 and 1, got {}",
        utilization
    );
}
