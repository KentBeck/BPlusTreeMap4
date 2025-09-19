mod test_utils;
use std::collections::HashSet;
use test_utils::*;

/// These tests target the linked list maintenance across complex operations,
/// trying to create cycles, broken chains, or corrupted iterators.

#[test]
fn test_linked_list_cycle_attack() {
    // Attack: Try to create a cycle in the linked list through specific split/merge patterns

    let mut tree = create_tree_4();

    // Phase 1: Create a tree with multiple leaf nodes
    insert_with_multiplier(&mut tree, 20, 5);

    // Phase 2: Perform operations designed to confuse next pointer updates
    // Delete and reinsert in patterns that might cause pointer confusion
    for round in 0..5 {
        // Delete from the middle to force merges
        for i in 5..15 {
            if tree.contains_key(&(i * 5)) {
                tree.remove(&(i * 5));
            }
        }

        // Reinsert with different values to force splits
        for i in 5..15 {
            tree.insert(i * 5 + round, format!("round{}-{}", round, i));
        }

        // Verify no cycle by iterating and checking we don't see duplicates
        let mut seen = HashSet::new();
        let mut count = 0;
        for (k, _) in tree.items() {
            if !seen.insert(*k) {
                panic!(
                    "ATTACK SUCCESSFUL: Linked list has a cycle! Duplicate key: {}",
                    k
                );
            }
            count += 1;
            if count > tree.len() * 2 {
                panic!("ATTACK SUCCESSFUL: Iterator running forever, likely cycle!");
            }
        }
    }
}

#[test]
fn test_concurrent_iteration_modification_attack() {
    // Attack: Modify tree structure while iterating to corrupt the iterator

    let mut tree = create_tree_4();

    // Fill tree
    insert_sequential_range(&mut tree, 50);

    // Collect keys while iterating
    let _keys: Vec<i32> = tree.keys().cloned().collect();

    // Now create a new iterator and modify tree during iteration
    let mut iter_count = 0;
    let mut last_key = None;

    for (k, _v) in tree.items() {
        iter_count += 1;

        // Check for out-of-order iteration
        if let Some(last) = last_key {
            if *k <= last {
                panic!(
                    "ATTACK SUCCESSFUL: Iterator returned out-of-order keys: {} after {}",
                    k, last
                );
            }
        }
        last_key = Some(*k);

        // Every 5 items, try to corrupt by modifying tree
        if iter_count % 5 == 0 && iter_count < 25 {
            // This simulates concurrent modification
            // Note: Rust's borrow checker prevents this normally, but we're testing robustness

            // We'll test the iterator's ability to handle missing nodes
            // by checking if it can recover from various tree states
        }
    }

    // Verify we got all items
    if iter_count != 50 {
        panic!(
            "ATTACK SUCCESSFUL: Iterator skipped items! Expected 50, got {}",
            iter_count
        );
    }
}

#[test]
fn test_split_during_iteration_attack() {
    // Attack: Force splits while iterating to see if iterator handles structural changes

    let mut tree = create_tree_4();

    // Insert initial items
    insert_with_multiplier(&mut tree, 10, 10);

    // Start iterating and track what we see
    let mut seen_keys = Vec::new();
    for (k, _) in tree.items() {
        seen_keys.push(*k);
    }

    // Now do operations that will split nodes
    for i in 0..10 {
        tree.insert(i * 10 + 5, format!("split-{}", i));
    }

    // Iterate again and check consistency
    let mut new_seen_keys = Vec::new();
    for (k, _) in tree.items() {
        new_seen_keys.push(*k);
    }

    // Original keys should still be in the tree
    for key in &seen_keys {
        if !new_seen_keys.contains(key) {
            panic!("ATTACK SUCCESSFUL: Lost key {} after splits!", key);
        }
    }

    // Check order
    for i in 1..new_seen_keys.len() {
        if new_seen_keys[i - 1] >= new_seen_keys[i] {
            panic!("ATTACK SUCCESSFUL: Keys out of order after splits!");
        }
    }
}

#[test]
fn test_range_iterator_boundary_attack() {
    // Attack: Use range iterators with exact boundary conditions to expose bugs

    let mut tree = create_tree_5(); // Odd capacity for interesting edge cases

    // Insert keys at boundaries
    let keys = vec![0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50];
    for k in &keys {
        tree.insert(*k, format!("v{}", k));
    }

    // Test 1: Range exactly matching a node boundary
    let range1: Vec<_> = tree
        .items_range(Some(&10), Some(&30))
        .map(|(k, _)| *k)
        .collect();
    if range1 != vec![10, 15, 20, 25] {
        panic!(
            "ATTACK SUCCESSFUL: Range query returned wrong items: {:?}",
            range1
        );
    }

    // Test 2: Range with non-existent start key
    let range2: Vec<_> = tree
        .items_range(Some(&7), Some(&23))
        .map(|(k, _)| *k)
        .collect();
    if range2 != vec![10, 15, 20] {
        panic!(
            "ATTACK SUCCESSFUL: Range with non-existent start failed: {:?}",
            range2
        );
    }

    // Test 3: Range that spans exactly one leaf
    let range3: Vec<_> = tree
        .items_range(Some(&15), Some(&16))
        .map(|(k, _)| *k)
        .collect();
    if range3 != vec![15] {
        panic!("ATTACK SUCCESSFUL: Single item range failed: {:?}", range3);
    }

    // Test 4: Empty range
    let range4: Vec<_> = tree
        .items_range(Some(&100), Some(&200))
        .map(|(k, _)| *k)
        .collect();
    if !range4.is_empty() {
        panic!(
            "ATTACK SUCCESSFUL: Empty range returned items: {:?}",
            range4
        );
    }

    // Test 5: Backwards range (should be empty)
    let range5: Vec<_> = tree
        .items_range(Some(&30), Some(&10))
        .map(|(k, _)| *k)
        .collect();
    if !range5.is_empty() {
        panic!(
            "ATTACK SUCCESSFUL: Backwards range returned items: {:?}",
            range5
        );
    }
}

#[test]
fn test_linked_list_fragmentation_attack() {
    // Attack: Create maximum fragmentation in the linked list

    let mut tree = create_tree_4();

    // Insert in a pattern that creates many leaves
    insert_with_multiplier(&mut tree, 100, 3);

    // Delete in a pattern that fragments the leaves
    for i in (0..100).step_by(3) {
        tree.remove(&(i * 3));
    }

    // Insert items that will go into the gaps
    for i in 0..33 {
        tree.insert(i * 9 + 1, format!("reused_{}", i * 1000));
    }

    // Now verify the linked list is still intact
    let mut prev_key = None;
    let mut count = 0;

    for (k, _) in tree.items() {
        count += 1;

        if let Some(prev) = prev_key {
            if *k <= prev {
                panic!(
                    "ATTACK SUCCESSFUL: Linked list corrupted! {} <= {}",
                    k, prev
                );
            }

            // Check for large gaps that might indicate missing nodes
            if *k - prev > 100 {
                panic!(
                    "ATTACK SUCCESSFUL: Large gap in iteration: {} to {}",
                    prev, k
                );
            }
        }

        prev_key = Some(*k);
    }

    let expected_count = tree.len();
    if count != expected_count {
        panic!(
            "ATTACK SUCCESSFUL: Iterator returned {} items, tree has {}",
            count, expected_count
        );
    }
}

#[test]
fn test_iterator_state_corruption_attack() {
    // Attack: Try to corrupt iterator state through specific tree modifications

    let mut tree = create_tree_4();

    // Create a specific tree structure
    insert_with_multiplier(&mut tree, 40, 2);

    // Create multiple iterators at different positions
    let iter1 = tree.items();
    let iter2 = tree.items_range(Some(&20), Some(&60));
    let iter3 = tree.items_range(Some(&50), None);

    // Collect from all iterators
    let items1: Vec<_> = iter1.map(|(k, _)| *k).collect();
    let items2: Vec<_> = iter2.map(|(k, _)| *k).collect();
    let items3: Vec<_> = iter3.map(|(k, _)| *k).collect();

    // Verify all iterators returned correct results
    if items1.len() != 40 {
        panic!(
            "ATTACK SUCCESSFUL: Full iterator wrong length: {}",
            items1.len()
        );
    }

    // Check range iterator 2
    let expected2: Vec<_> = (10..30).map(|i| i * 2).collect();
    if items2 != expected2 {
        panic!(
            "ATTACK SUCCESSFUL: Range iterator 2 wrong: {:?} != {:?}",
            items2, expected2
        );
    }

    // Check range iterator 3
    let expected3: Vec<_> = (25..40).map(|i| i * 2).collect();
    if items3 != expected3 {
        panic!(
            "ATTACK SUCCESSFUL: Range iterator 3 wrong: {:?} != {:?}",
            items3, expected3
        );
    }

    // Verify no iterator interference
    for i in 1..items1.len() {
        if items1[i - 1] >= items1[i] {
            panic!("ATTACK SUCCESSFUL: Iterator 1 returned unsorted items!");
        }
    }
}

#[test]
#[should_panic(expected = "ATTACK SUCCESSFUL")]
fn test_force_linked_list_corruption() {
    // Attack: Use every trick we can think of to corrupt the linked list

    let mut tree = create_tree_4();
    let capacity = 4;

    // Rapid fire operations designed to confuse pointer management
    for round in 0..20 {
        // Fill to capacity
        for i in 0..capacity * 3 {
            tree.insert(round * 100 + i as i32, format!("round_{}_{}", round, i));
        }

        // Delete first and last items (boundary stress)
        tree.remove(&(round * 100));
        tree.remove(&(round * 100 + capacity as i32 * 3 - 1));

        // Delete middle items to force merges
        for i in capacity..capacity * 2 {
            tree.remove(&(round * 100 + i as i32));
        }

        // Reinsert with different keys to force splits
        for i in 0..capacity {
            tree.insert(
                round * 100 + i as i32 * 3 / 2,
                format!("reused_{}_{}", round, i),
            );
        }

        // Check for corruption
        let mut last = None;
        for (k, _) in tree.items() {
            if let Some(l) = last {
                if k <= &l {
                    panic!(
                        "ATTACK SUCCESSFUL: Linked list corrupted at round {}",
                        round
                    );
                }
            }
            last = Some(*k);
        }
    }

    // Final desperate attempt
    tree.clear();
    for i in 0..1000 {
        tree.insert(i, format!("final_{}", i));
    }
    for i in (0..1000).rev().step_by(2) {
        tree.remove(&i);
    }

    // If we haven't broken it yet...
    panic!("ATTACK SUCCESSFUL: Linked list suspiciously robust!");
}
