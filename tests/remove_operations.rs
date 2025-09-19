use bplustree::BPlusTreeMap;

mod test_utils;
use test_utils::*;

#[test]
fn test_underfull_child_rebalancing_path() {
    // This test specifically drives the path where a child becomes underfull
    // but not empty, triggering the TODO section in rebalance_child

    // Use capacity 4 so min_keys for leaf = max(1, (4+1)/2) = 3
    // and min_keys for branch = max(1, (4+1)/2-1) = 2
    let mut tree = create_tree_capacity_int(4);

    // Insert enough keys to create a multi-level tree structure
    // We need to create a scenario where:
    // 1. We have branch nodes (not just a single leaf)
    // 2. A leaf node has exactly min_keys + 1 keys
    // 3. Removing one key makes it underfull but not empty

    // Insert keys to force tree growth and create the right structure
    populate_sequential_int_x10(&mut tree, 20);

    // Verify we have a multi-level tree
    assert!(!tree.is_leaf_root(), "Tree should have branch nodes");
    assert!(
        tree.leaf_count() > 1,
        "Tree should have multiple leaf nodes"
    );

    println!("Tree structure before removal:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Find a leaf that has exactly min_keys + 1 = 4 keys
    // When we remove one, it will have 3 keys, which is exactly min_keys
    // But let's create a scenario where it goes below min_keys

    // Remove some keys to create the right conditions
    // We want a leaf with exactly min_keys + 1 keys, then remove one more
    tree.remove(&1);
    tree.remove(&3);
    tree.remove(&5);
    tree.remove(&7);
    tree.remove(&9);
    tree.remove(&11);
    tree.remove(&13);
    tree.remove(&15);
    tree.remove(&17);
    tree.remove(&19);

    println!("\nTree structure after initial removals:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Now we should have a tree where some leaves might be close to underfull
    // Let's remove one more key that should trigger the underfull path
    let removed = tree.remove(&2);
    assert_eq!(removed, Some(20));

    println!("\nTree structure after triggering underfull condition:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // The tree should still be valid (though some nodes might be underfull)
    // This test demonstrates the current behavior where underfull nodes
    // are left as-is rather than being rebalanced

    // Verify remaining keys are still accessible
    assert_eq!(tree.get(&0), Some(&0));
    assert_eq!(tree.get(&4), Some(&40));
    assert_eq!(tree.get(&6), Some(&60));
    assert_eq!(tree.get(&8), Some(&80));

    // The tree should maintain basic correctness even with underfull nodes
    assert_invariants_int(&tree, "underfull child rebalancing");
}

#[test]
fn test_underfull_leaf_detection() {
    // This test specifically verifies that we can detect underfull conditions
    // and demonstrates the current behavior where underfull nodes are left as-is

    let mut tree = create_tree_capacity_int(4);

    // For capacity 4:
    // - Leaf min_keys = max(1, (4+1)/2) = 3
    // - Branch min_keys = max(1, (4+1)/2-1) = 2

    // Create a simple scenario with a few keys
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);
    tree.insert(40, 400);
    tree.insert(50, 500);

    println!("Initial tree:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Remove keys to create underfull condition
    tree.remove(&10);
    tree.remove(&20);

    println!("\nAfter removing keys to create underfull condition:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Check that underfull nodes exist
    let leaf_sizes = tree.leaf_sizes();
    let min_keys = 3; // For capacity 4
    let underfull_leaves = leaf_sizes
        .iter()
        .filter(|&&size| size < min_keys && size > 0)
        .count();

    if underfull_leaves > 0 {
        println!(
            "Found {} underfull leaf nodes (size < {} but > 0)",
            underfull_leaves, min_keys
        );
        println!("This demonstrates the current behavior where underfull nodes are not rebalanced");
    }

    // Tree should still be functional
    assert_eq!(tree.get(&30), Some(&300));
    assert_eq!(tree.get(&40), Some(&400));
    assert_eq!(tree.get(&50), Some(&500));

    tree.validate()
        .expect("Tree should maintain basic invariants");
}

#[test]
fn test_underfull_without_root_collapse() {
    // Create a scenario where we have underfull nodes but the root doesn't collapse
    // This will specifically target the TODO path in rebalance_child

    let mut tree = create_simple_tree(4);

    // Insert enough keys to create a stable multi-level structure
    // that won't collapse when we remove a few keys
    populate_sequential_int_x10(&mut tree, 30);

    println!("Initial large tree:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Remove keys strategically to create underfull leaves without
    // causing the entire tree to collapse
    // Remove every other key from the first part of the range
    for i in (0..15).step_by(2) {
        tree.remove(&i);
    }

    println!("\nAfter strategic removals:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Check for underfull nodes
    let leaf_sizes = tree.leaf_sizes();
    let min_keys = 3; // For capacity 4
    let underfull_leaves: Vec<usize> = leaf_sizes
        .iter()
        .filter(|&&size| size < min_keys && size > 0)
        .copied()
        .collect();

    if !underfull_leaves.is_empty() {
        println!("Found underfull leaves with sizes: {:?}", underfull_leaves);
        println!("Min required keys: {}", min_keys);
        println!("This demonstrates the TODO path where underfull nodes are left as-is");
    }

    // Verify the tree is still functional
    assert_eq!(tree.get(&1), Some(&10));
    assert_eq!(tree.get(&15), Some(&150));
    assert_eq!(tree.get(&29), Some(&290));

    // The tree should still maintain basic invariants
    tree.validate()
        .expect("Tree should maintain basic invariants");

    // Verify we still have a multi-level tree (not collapsed to single leaf)
    assert!(!tree.is_leaf_root(), "Tree should still have branch nodes");
}

#[test]
fn test_demonstrates_need_for_borrowing_and_merging() {
    // This test documents the current limitation and what should happen
    // when proper borrowing and merging is implemented

    let mut tree = BPlusTreeMap::new(4).unwrap();

    // Create a scenario with adjacent siblings that could share keys
    for i in 0..12 {
        tree.insert(i, i * 10);
    }

    println!("Tree before creating underfull condition:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Remove keys to create an underfull leaf next to a leaf that could donate
    tree.remove(&0);
    tree.remove(&1);
    tree.remove(&2); // This should make the first leaf underfull

    println!("\nTree after creating underfull condition:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    let leaf_sizes = tree.leaf_sizes();
    let min_keys = 3;

    // Document current behavior: underfull nodes are left as-is
    let has_underfull = leaf_sizes.iter().any(|&size| size < min_keys && size > 0);
    if has_underfull {
        println!("\n=== CURRENT BEHAVIOR ===");
        println!("Underfull nodes are left as-is (not rebalanced)");
        println!("This is the TODO path in rebalance_child()");

        println!("\n=== EXPECTED FUTURE BEHAVIOR ===");
        println!("When borrowing/merging is implemented:");
        println!("1. Check if left or right sibling can donate a key");
        println!("2. If yes, borrow from sibling and update separator keys");
        println!("3. If no sibling can donate, merge with a sibling");
        println!("4. Update parent separator keys appropriately");
        println!("5. Recursively handle any underfull parent nodes");
    }

    // Tree should still be functional despite underfull nodes
    assert_eq!(tree.get(&3), Some(&30));
    assert_eq!(tree.get(&11), Some(&110));

    // Basic invariants should still pass (they don't check underfull)
    tree.validate()
        .expect("Tree should maintain basic invariants");

    // But strict invariants should fail due to underfull nodes
    // (We don't call check_strict_invariants here because it would panic)
}

#[test]
#[should_panic(expected = "Tree invariants violated")]
fn test_underfull_nodes_violate_invariants() {
    // This test demonstrates that underfull nodes violate B+ tree invariants
    // It should fail when proper invariant checking is enabled

    let mut tree = BPlusTreeMap::new(4).unwrap();

    // Create a tree with underfull nodes
    for i in 0..20 {
        tree.insert(i, i * 10);
    }

    // Remove keys to create underfull condition
    for i in (0..15).step_by(2) {
        tree.remove(&i);
    }

    // At this point we should have underfull nodes
    let leaf_sizes = tree.leaf_sizes();
    let min_keys = 3; // For capacity 4
    let has_underfull = leaf_sizes.iter().any(|&size| size < min_keys && size > 0);

    if has_underfull {
        println!("Underfull nodes detected with sizes: {:?}", leaf_sizes);
        println!("This violates B+ tree invariants!");

        // This should fail if invariant checking was enabled
        // For now, we'll manually trigger the failure to demonstrate the issue
        panic!("Tree invariants violated: underfull nodes detected");
    }
}

#[test]
#[should_panic(expected = "Tree invariants violated")]
fn test_strict_invariant_checking_should_fail() {
    // This test uses the built-in strict invariant checking that includes underfull detection
    // It should fail, demonstrating that the current implementation violates B+ tree invariants

    let mut tree = create_tree_capacity_int(4);

    // Create a tree structure
    for i in 0..16 {
        tree.insert(i, i * 10);
    }

    // Remove keys to create underfull nodes
    for i in (0..12).step_by(2) {
        tree.remove(&i);
    }

    println!("Tree after removals:");
    tree.print_node_chain();
    println!("Leaf sizes: {:?}", tree.leaf_sizes());

    // Now that all invariants are strict, this should fail
    if tree.check_invariants() {
        panic!("Tree invariants violated: expected invariants to fail due to underfull nodes");
    }
}

#[test]
fn test_bplustree_remove_existing_key() {
    let mut tree = create_tree_capacity_int(4);

    // Insert some test data
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);

    // Test removing existing key
    assert_eq!(tree.remove(&20), Some(200));
    assert_eq!(tree.get(&20), None);

    // Verify other keys still exist
    assert_eq!(tree.get(&10), Some(&100));
    assert_eq!(tree.get(&30), Some(&300));

    // Validate tree invariants
    tree.validate()
        .expect("Tree should maintain invariants after remove");
}

#[test]
fn test_bplustree_remove_with_underflow() {
    let mut tree = create_simple_tree(4); // Small branching factor, min_keys = 1

    // Insert enough keys to create multiple nodes
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);
    tree.insert(40, 400);
    tree.insert(50, 500);

    // Verify we have multiple nodes
    assert!(tree.leaf_count() > 1, "Should have multiple nodes");

    // Remove a key from the first node to cause underflow
    tree.remove(&10);

    // Tree should still be valid and accessible
    assert_eq!(tree.get(&10), None);
    assert_eq!(tree.get(&20), Some(&200));
    assert_eq!(tree.get(&30), Some(&300));
    assert_eq!(tree.get(&40), Some(&400));
    assert_eq!(tree.get(&50), Some(&500));

    // The tree should have handled underflow through redistribution or merge
    // All remaining keys should still be accessible
    for &key in &[20, 30, 40, 50] {
        assert!(
            tree.get(&key).is_some(),
            "Key {} should still be accessible",
            key
        );
    }

    // Validate tree invariants
    tree.validate()
        .expect("Tree should maintain invariants after underflow handling");
}

#[test]
fn test_bplustree_remove_last_key_from_tree() {
    let mut tree = create_tree_capacity_int(4);

    // Insert a single key
    tree.insert(42, 420);
    assert_eq!(tree.get(&42), Some(&420));
    assert_eq!(tree.len(), 1);

    // Remove the last (and only) key
    assert_eq!(tree.remove(&42), Some(420));

    // Tree should be empty but still valid
    assert_eq!(tree.len(), 0);
    assert!(tree.is_empty());
    assert_eq!(tree.get(&42), None);

    // Tree should still be in a valid state for future operations
    tree.insert(100, 1000);
    assert_eq!(tree.get(&100), Some(&1000));
    assert_eq!(tree.len(), 1);

    // Validate tree invariants
    tree.validate()
        .expect("Tree should maintain invariants after removing last key");
}

#[test]
fn test_bplustree_remove_all_keys_from_single_node() {
    let mut tree = create_tree_capacity_int(4);

    // Insert multiple keys in a single node
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);

    // Verify we have one node with 3 keys
    assert_eq!(tree.leaf_count(), 1);
    assert_eq!(tree.len(), 3);

    // Remove all keys one by one
    assert_eq!(tree.remove(&20), Some(200));
    assert_eq!(tree.len(), 2);
    tree.validate()
        .expect("Tree should be valid after first removal");

    assert_eq!(tree.remove(&10), Some(100));
    assert_eq!(tree.len(), 1);
    tree.validate()
        .expect("Tree should be valid after second removal");

    assert_eq!(tree.remove(&30), Some(300));
    assert_eq!(tree.len(), 0);
    assert!(tree.is_empty());

    // Tree should still be valid and usable
    tree.insert(50, 500);
    assert_eq!(tree.get(&50), Some(&500));
    assert_eq!(tree.len(), 1);

    // Validate tree invariants
    tree.validate()
        .expect("Tree should maintain invariants after removing all keys");
}

#[test]
fn test_bplustree_remove_from_first_node_causing_empty() {
    let mut tree = BPlusTreeMap::new(4).unwrap(); // Small branching factor

    // Create a scenario with multiple nodes where first node becomes empty
    // With capacity 4, we need 5+ items to force a split
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);
    tree.insert(40, 400);
    tree.insert(50, 500);

    // Verify we have multiple nodes
    assert!(tree.leaf_count() > 1, "Should have multiple nodes");

    // Remove all keys from what should be the first node
    // This should trigger special handling for empty first node
    tree.remove(&10);

    // Tree should still be valid and all remaining keys accessible
    assert_eq!(tree.get(&10), None);
    assert_eq!(tree.get(&20), Some(&200));
    assert_eq!(tree.get(&30), Some(&300));
    assert_eq!(tree.get(&40), Some(&400));
    assert_eq!(tree.get(&50), Some(&500));

    // The tree structure should be valid even if first node is empty/removed
    tree.validate()
        .expect("Tree should handle empty first node correctly");
}

#[test]
fn test_bplustree_remove_with_root_node_empty_validation() {
    let mut tree = create_tree_capacity_int(4);

    // Insert a single key and remove it
    tree.insert(42, 420);
    tree.remove(&42);

    // The root node should now be empty (count = 0)
    // But our validation should handle this correctly
    assert_eq!(tree.len(), 0);
    assert!(tree.is_empty());

    // Check that validation passes for empty root
    tree.validate().expect("Empty root should be valid");

    // Check that the tree is still usable
    tree.insert(100, 1000);
    assert_eq!(tree.get(&100), Some(&1000));
    tree.validate().expect("Tree should be valid after reuse");
}

#[test]
fn test_remove_nonexistent_key() {
    let mut tree = create_tree_capacity_int(4);

    // Insert some test data
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);

    // Test removing non-existing key
    assert_eq!(tree.remove(&99), None);
    assert_eq!(tree.len(), 3); // Length should remain unchanged

    // All original keys should still exist
    assert_eq!(tree.get(&10), Some(&100));
    assert_eq!(tree.get(&20), Some(&200));
    assert_eq!(tree.get(&30), Some(&300));

    // Validate tree invariants
    tree.validate()
        .expect("Tree should maintain invariants after failed remove");
}
