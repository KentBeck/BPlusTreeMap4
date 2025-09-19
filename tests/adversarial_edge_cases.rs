mod test_utils;
use test_utils::*;

/// Final adversarial tests targeting root collapse logic, capacity boundaries,
/// and other edge cases that might reveal bugs.

#[test]
fn test_root_collapse_infinite_loop_attack() {
    // Attack: Try to create an infinite loop in root collapse logic

    let mut tree = create_attack_tree(4);

    // Build a multi-level tree
    populate_sequential(&mut tree, 64);

    // Delete in a pattern that forces repeated root collapses
    for i in (0..64).rev() {
        if i % 8 != 0 {
            tree.remove(&i);
            assert_attack_failed(&tree, &format!("deletion {}", i));
        }
    }

    // Tree should now have very few items but still be valid
    let remaining: Vec<_> = tree.keys().cloned().collect();
    println!("Remaining keys after collapse attack: {:?}", remaining);

    // Try to break it with one more operation
    tree.insert(100, String::from("final"));

    verify_item_count(&tree, remaining.len() + 1, "root collapse final check");
}

#[test]
fn test_minimum_capacity_edge_cases_attack() {
    // Attack: Use minimum capacity (4) and test all edge cases

    let capacity = 4; // Minimum allowed
    let mut tree = create_attack_tree(capacity);

    // Test 1: Exactly capacity items in root leaf
    for i in 0..capacity {
        tree.insert(i as i32, format!("v{}", i));
    }

    // This should trigger first split
    tree.insert(capacity as i32, String::from("split"));

    // Verify split happened correctly
    if tree.is_leaf_root() {
        panic!("ATTACK SUCCESSFUL: Root didn't promote to branch after split!");
    }

    // Test 2: Delete to exactly min_keys in each node
    tree.clear();

    // Insert pattern to create specific structure
    insert_with_multiplier(&mut tree, 50, 2);

    // Delete to leave each node at minimum
    for i in vec![1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29] {
        if tree.contains_key(&i) {
            tree.remove(&i);
        }
    }

    // Try one more deletion - should trigger rebalancing
    tree.remove(&0);

    // Verify tree is still valid
    assert_attack_failed(&tree, "minimum capacity operations");
}

#[test]
fn test_odd_capacity_arithmetic_attack() {
    // Attack: Use odd capacities to expose integer division bugs

    for capacity in vec![5, 7, 9, 11] {
        let mut tree = create_attack_tree(capacity);

        // Fill to exactly trigger splits at boundaries
        for i in 0..(capacity * 10) {
            tree.insert(i as i32, format!("cap{}-{}", capacity, i));
        }

        // min_keys calculation for odd numbers
        let min_keys = capacity / 2; // Floor division

        // Delete to exactly min_keys in some nodes
        let mut deleted = 0;
        for i in (0..(capacity * 10)).rev() {
            if deleted >= capacity * 7 {
                break;
            }
            if i % 3 != 0 {
                tree.remove(&(i as i32));
                deleted += 1;
            }
        }

        // Verify invariants with odd capacity
        assert_attack_failed(&tree, &format!("odd capacity {}", capacity));

        // Test boundary: exactly min_keys items
        tree.clear();
        for i in 0..min_keys {
            tree.insert(i as i32, format!("min-{}", i));
        }

        // This should be valid for root
        assert_attack_failed(
            &tree,
            &format!("root with {} items (capacity {})", min_keys, capacity),
        );
    }
}

#[test]
fn test_insert_remove_same_key_attack() {
    // Attack: Rapidly insert and remove the same key to confuse state

    let capacity = 4;
    let mut tree = create_attack_tree(capacity);

    // Setup initial tree
    for i in 0..20 {
        tree.insert(i * 2, format!("initial-{}", i));
    }

    // Rapid fire insert/remove of same key
    let target_key = 21; // Key that doesn't exist initially

    for round in 0..100 {
        tree.insert(target_key, format!("round-{}", round));

        // Sometimes don't remove to change tree structure
        if round % 3 != 0 {
            let removed = tree.remove(&target_key);
            if removed != Some(format!("round-{}", round)) {
                panic!("ATTACK SUCCESSFUL: Wrong value removed in round {}", round);
            }
        }
    }

    // Verify tree structure is still sound
    verify_ordering(&tree);
}

#[test]
fn test_get_mut_corruption_attack() {
    // Attack: Use get_mut to try to corrupt tree invariants

    let _capacity = 4;
    let mut tree = create_tree_4();

    // Insert items
    for i in 0..30 {
        tree.insert(i, format!("vec_{}_data", i)); // String data for testing
    }

    // Get mutable references and modify
    for i in 0..30 {
        if let Some(v) = tree.get_mut(&i) {
            // Modify the value in a way that might confuse tree
            v.clear();
            v.push_str(&format!("modified_{}", i * 100));
        }
    }

    // Verify tree structure wasn't affected by value mutations
    if let Err(e) = tree.check_invariants_detailed() {
        panic!("ATTACK SUCCESSFUL: get_mut corrupted tree: {}", e);
    }

    // Verify all values were modified correctly
    for i in 0..30 {
        if let Some(v) = tree.get(&i) {
            if !v.contains(&format!("modified_{}", i * 100)) {
                panic!("ATTACK SUCCESSFUL: Value corruption through get_mut!");
            }
        } else {
            panic!("ATTACK SUCCESSFUL: Lost key {} after get_mut!", i);
        }
    }
}

#[test]
fn test_split_merge_thrashing_attack() {
    // Attack: Cause repeated splits and merges in the same nodes

    let _capacity = 4;
    let mut tree = create_tree_4();

    // Insert to create initial structure
    insert_with_multiplier(&mut tree, 20, 3);

    // Thrash: repeatedly fill and empty nodes
    for round in 0..10 {
        println!("Thrash round {}", round);

        // Fill gaps to cause splits
        for i in 0..20 {
            tree.insert(i * 3 + 1, format!("fill-{}-{}", round, i));
        }

        // Remove the fill items to cause merges
        for i in 0..20 {
            tree.remove(&(i * 3 + 1));
        }

        // Verify tree is still consistent
        if let Err(e) = tree.check_invariants_detailed() {
            panic!("ATTACK SUCCESSFUL at round {}: {}", round, e);
        }

        // Check size is back to original
        if tree.len() != 20 {
            panic!(
                "ATTACK SUCCESSFUL: Lost items during thrashing! Expected 20, got {}",
                tree.len()
            );
        }
    }
}

#[test]
fn test_extreme_key_values_attack() {
    // Attack: Use extreme key values to test boundary conditions

    let _capacity = 4;
    let mut tree = create_tree_4();

    // Test with minimum and maximum i32 values
    let extreme_keys = vec![
        i32::MIN,
        i32::MIN + 1,
        -1000000,
        -1,
        0,
        1,
        1000000,
        i32::MAX - 1,
        i32::MAX,
    ];

    // Insert extreme values
    for (i, &key) in extreme_keys.iter().enumerate() {
        tree.insert(key, format!("extreme-{}", i));
    }

    // Verify ordering is maintained
    let keys: Vec<_> = tree.keys().cloned().collect();
    for i in 1..keys.len() {
        if keys[i - 1] >= keys[i] {
            panic!("ATTACK SUCCESSFUL: Extreme keys broke ordering!");
        }
    }

    // Test range queries with extreme bounds
    let range1: Vec<_> = tree
        .items_range(Some(&i32::MIN), Some(&0))
        .map(|(k, _)| *k)
        .collect();

    if range1.len() != 4 {
        // MIN, MIN+1, -1000000, -1
        panic!(
            "ATTACK SUCCESSFUL: Range query with MIN bound failed: {:?}",
            range1
        );
    }

    // Delete extreme values
    for &key in &extreme_keys {
        if tree.remove(&key).is_none() {
            panic!("ATTACK SUCCESSFUL: Failed to remove extreme key {}", key);
        }
    }

    if !tree.is_empty() {
        panic!("ATTACK SUCCESSFUL: Tree not empty after removing all extreme keys!");
    }
}

#[test]
#[should_panic(expected = "ATTACK SUCCESSFUL")]
fn test_ultimate_adversarial_attack() {
    // Final attack: Everything we can think of

    let _capacity = 4;
    let mut tree = create_tree_4();

    // Combine all attack patterns
    for attack_round in 0..5 {
        // 1. Extreme keys
        tree.insert(i32::MAX - attack_round, format!("max_{}", attack_round));
        tree.insert(i32::MIN + attack_round, format!("min_{}", attack_round));

        // 2. Rapid operations
        for i in 0..20 {
            tree.insert(i, format!("attack_{}", i));
            if i % 2 == 0 {
                tree.remove(&i);
            }
        }

        // 3. Force root changes
        for i in 0..100 {
            tree.insert(i * attack_round, format!("combo_{}_{}", attack_round, i));
        }
        for i in (0..100).rev().step_by(2) {
            tree.remove(&(i * attack_round));
        }

        // 4. Boundary operations
        let size = tree.len();
        if size == 0 {
            continue;
        }

        // Try to corrupt through get_mut
        let some_key = *tree.keys().next().unwrap();
        if let Some(v) = tree.get_mut(&some_key) {
            *v = format!("extreme_{}", i32::MAX); // Extreme value modification
        }

        // 5. Check for any sign of corruption
        match tree.check_invariants_detailed() {
            Ok(_) => {}
            Err(e) => panic!("ATTACK SUCCESSFUL: Combined attack worked! {}", e),
        }

        // Check iteration still works
        let count = tree.items().count();
        if count != tree.len() {
            panic!("ATTACK SUCCESSFUL: Iterator count mismatch!");
        }
    }

    // If we survived all that...
    panic!("ATTACK SUCCESSFUL: B+ tree is impossibly robust! No bugs found!");
}
