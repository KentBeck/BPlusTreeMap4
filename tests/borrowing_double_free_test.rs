use bplustree::BPlusTreeMap;

#[test]
fn test_borrowing_operations_memory_safety() {
    // Test that borrowing operations don't cause double-free issues
    let mut tree = BPlusTreeMap::new(4).unwrap();

    // Insert enough items to create a multi-level tree that will require borrowing
    for i in 0..20 {
        tree.insert(i, format!("value_{}", i));
    }

    // Remove items to trigger borrowing operations
    for i in (0..20).step_by(2) {
        tree.remove(&i);
    }

    // The tree should still be valid
    for i in (1..20).step_by(2) {
        assert_eq!(tree.get(&i), Some(&format!("value_{}", i)));
    }

    // Drop the tree - this should not cause double-free
    drop(tree);
}

#[test]
fn test_leaf_borrowing_from_left() {
    let mut tree = BPlusTreeMap::new(4).unwrap();

    // Create a scenario that will trigger left leaf borrowing
    // Insert keys to create multiple leaves
    for i in 0..12 {
        tree.insert(i, i * 10);
    }

    // Remove keys from the middle leaf to make it underfull
    tree.remove(&4);
    tree.remove(&5);

    // This should trigger borrowing from left leaf
    tree.remove(&6);

    // Verify tree is still consistent
    assert_eq!(tree.get(&0), Some(&0));
    assert_eq!(tree.get(&1), Some(&10));
    assert_eq!(tree.get(&7), Some(&70));

    // Drop should not cause double-free
    drop(tree);
}

#[test]
fn test_leaf_borrowing_from_right() {
    let mut tree = BPlusTreeMap::new(4).unwrap();

    // Create a scenario that will trigger right leaf borrowing
    for i in 0..12 {
        tree.insert(i, i * 10);
    }

    // Remove keys from the first leaf to make it underfull
    tree.remove(&1);
    tree.remove(&2);

    // This should trigger borrowing from right leaf
    tree.remove(&3);

    // Verify tree is still consistent
    assert_eq!(tree.get(&0), Some(&0));
    assert_eq!(tree.get(&4), Some(&40));
    assert_eq!(tree.get(&11), Some(&110));

    // Drop should not cause double-free
    drop(tree);
}

#[test]
fn test_branch_borrowing_operations() {
    let mut tree = BPlusTreeMap::new(4).unwrap();

    // Create a deeper tree that will have branch borrowing
    for i in 0..50 {
        tree.insert(i, i * 10);
    }

    // Remove many items to trigger branch rebalancing and borrowing
    for i in (10..30).step_by(1) {
        tree.remove(&i);
    }

    // Verify remaining items are still accessible
    for i in 0..10 {
        assert_eq!(tree.get(&i), Some(&(i * 10)));
    }
    for i in 30..50 {
        assert_eq!(tree.get(&i), Some(&(i * 10)));
    }

    // Drop should not cause double-free
    drop(tree);
}

#[test]
fn test_mixed_operations_stress() {
    let mut tree = BPlusTreeMap::new(6).unwrap();

    // Stress test with mixed insert/remove operations
    for round in 0..5 {
        let base = round * 20;

        // Insert a batch
        for i in 0..20 {
            tree.insert(base + i, (base + i) * 10);
        }

        // Remove every third item to trigger various rebalancing scenarios
        for i in (0..20).step_by(3) {
            tree.remove(&(base + i));
        }
    }

    // Final cleanup - remove more items
    for i in (0..100).step_by(7) {
        tree.remove(&i);
    }

    // Tree should still be functional
    assert!(tree.len() > 0);

    // Drop should not cause double-free
    drop(tree);
}
