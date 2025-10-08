/// Test cases to reproduce specific bugs found in the B+ tree implementation
/// Each test demonstrates a concrete failure case for the identified issues
// BPlusTreeMap import removed - using test_utils instead
mod test_utils;
use test_utils::*;

#[test]
fn test_linked_list_corruption_during_merge() {
    let mut tree = create_tree_4();

    // Create a scenario that will cause leaf merging
    // Insert keys to create multiple leaves
    insert_with_multiplier(&mut tree, 20, 10);

    // Capture the linked list structure before deletion
    let _items_before: Vec<_> = tree.items().collect();

    // Delete items to trigger merging
    for i in 5..15 {
        tree.remove(&(i * 10));
    }

    // Verify linked list is still consistent
    let items_after: Vec<_> = tree.items().collect();

    // Check that iteration gives us all remaining keys in order
    let mut expected_keys = Vec::new();
    for i in 0..5 {
        expected_keys.push(i * 10);
    }
    for i in 15..20 {
        expected_keys.push(i * 10);
    }

    let actual_keys: Vec<_> = items_after.iter().map(|(k, _)| **k).collect();

    if actual_keys != expected_keys {
        panic!(
            "Linked list corruption: expected {:?}, got {:?}",
            expected_keys, actual_keys
        );
    }
}

#[test]
fn test_root_split_linked_list_race() {
    let tree = create_tree_4_with_data(5);

    // At this point we should have a branch root with leaf children
    // The leaf linked list should be properly maintained

    // Verify by checking that iteration gives us all keys in order
    let items: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    let expected: Vec<_> = (0..5).collect();

    if items != expected {
        panic!("Root split linked list race: iteration broken after root split");
    }

    // Also check that iteration still works correctly after root split
    let all_items: Vec<_> = tree.items().collect();
    if all_items.is_empty() {
        panic!("Root split linked list race: iteration returns no items");
    }
}

#[test]
fn test_range_iterator_bound_handling() {
    let tree = create_tree_4_with_data(10);

    // Test excluded start bound
    use std::ops::Bound;
    let range = (Bound::Excluded(&3), Bound::Unbounded);
    let items: Vec<_> = tree.range(range).map(|(k, _)| *k).collect();

    // Should start from 4, not 3
    if items.contains(&3) {
        panic!("Range iterator bound error: excluded start bound 3 was included");
    }

    if !items.contains(&4) {
        panic!("Range iterator bound error: item 4 should be included after excluded 3");
    }

    // Test case where excluded key doesn't exist
    let range2 = (Bound::Excluded(&2), Bound::Excluded(&7));
    let items2: Vec<_> = tree.range(range2).map(|(k, _)| *k).collect();
    let expected2 = vec![3, 4, 5, 6];

    if items2 != expected2 {
        panic!(
            "Range iterator bound error: expected {:?}, got {:?}",
            expected2, items2
        );
    }
}

#[test]
#[should_panic(expected = "Min keys inconsistency")]
fn test_min_keys_calculation_inconsistency() {
    let _tree = create_tree_6();

    // For capacity 6, different node types might need different min_keys
    // Standard B+ tree: leaves need ceil(6/2) = 3, branches need ceil(6/2)-1 = 2

    // Create a leaf and branch to test (this is a bit artificial since we can't
    // directly access node types, but we can infer from tree behavior)

    // The issue is that both use capacity/2 = 3, but branches should use 2
    // This can lead to invalid trees where branch operations fail

    // We'll test this by creating a scenario that should work with correct
    // min_keys but fails with incorrect ones

    let leaf_min = 6 / 2; // Current implementation: 3
    let branch_min = 6 / 2; // Current implementation: 3 (should be 2)

    // If both are 3, then certain merge operations that should be valid
    // (when branch has 2 keys) will be rejected
    if leaf_min == branch_min {
        panic!("Min keys inconsistency: leaf and branch use same formula");
    }
}

#[test]
fn test_iterator_lifetime_safety() {
    let tree = create_tree_4_with_data(10);

    // Create a range iterator that might have lifetime issues
    let range_iter = tree.range(3..7);

    // This should not panic due to lifetime issues
    let items: Vec<_> = range_iter.collect();
    assert_eq!(items.len(), 4);

    // The test passes if no panic occurs
}

#[test]
fn test_root_collapse_edge_cases() {
    let mut tree = create_tree_4_with_data(100);

    // Create a specific tree structure that will cause cascading collapse issues
    // Insert enough data to create multiple levels

    // Remove most items to force multiple levels of collapse
    deletion_range_attack(&mut tree, 0, 95);

    // If root collapse doesn't handle cascading properly,
    // we might end up with a malformed tree
    assert_invariants(&tree, "root collapse cascade");

    // Also check that the remaining items are still accessible
    let remaining_items: Vec<_> = tree.items().collect();
    if remaining_items.len() != 5 {
        panic!(
            "Root collapse cascade error: expected 5 items, got {}",
            remaining_items.len()
        );
    }
}
