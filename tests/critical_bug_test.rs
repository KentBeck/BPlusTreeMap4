/// Test to verify linked list integrity during merge operations
/// These tests ensure proper linked list maintenance during deletions
use bplustree::BPlusTreeMap;

mod test_utils;

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
