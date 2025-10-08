/// Simplified tests to demonstrate specific bugs in the B+ tree implementation
mod test_utils;
use test_utils::*;

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
