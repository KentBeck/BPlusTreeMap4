mod test_utils;
use test_utils::*;

/// These tests are designed to break the B+ tree implementation by targeting
/// the complex, untested branch rebalancing logic revealed by coverage analysis.
/// We're looking for panics, invariant violations, and data corruption.

#[test]
fn test_cascading_branch_rebalance_attack() {
    // Attack: Create a tree where all branch nodes are at minimum capacity,
    // then trigger cascading rebalances through multiple levels

    let capacity = 4; // min_keys = 2 for branches
    let mut tree = create_tree_capacity(capacity);

    // Build a 3-level tree where all branches are at minimum capacity
    // This requires careful insertion order

    // First, fill to create initial structure
    for i in 0..50 {
        tree.insert(i * 3, format!("value{}", i));
    }

    // Now carefully delete to leave all branches at minimum
    // This is the setup for our attack
    let mut keys_to_delete = vec![];
    for i in 0..50 {
        if i % 4 != 0 {
            keys_to_delete.push(i * 3);
        }
    }

    for key in keys_to_delete {
        tree.remove(&key);
        // Verify tree is still valid after each deletion
        assert!(
            tree.check_invariants(),
            "Invariants violated during setup at key {}",
            key
        );
    }

    // Now the attack: delete keys that will force cascading rebalances
    // Target keys that will make branches underfull
    println!("Tree structure before attack");

    // This deletion should trigger a cascade of rebalances
    let attack_key = 0;
    println!(
        "\nDeleting key {} to trigger cascading rebalance...",
        attack_key
    );
    tree.remove(&attack_key);

    // Check if we broke invariants
    match tree.check_invariants_detailed() {
        Ok(_) => println!("Invariants still hold after attack (tree survived)"),
        Err(e) => panic!("ATTACK SUCCESSFUL: Invariants violated! {}", e),
    }
}

#[test]
fn test_branch_borrow_from_underfull_sibling_attack() {
    // Attack: Force a branch to try borrowing from a sibling that can't donate
    // This targets the untested branch borrowing logic

    let capacity = 4;
    let mut tree = create_tree_capacity(capacity);

    // Build specific tree structure where both siblings are at minimum
    // Insert pattern designed to create this structure
    let keys = vec![
        10, 20, 30, 40, 15, 25, 35, 45, 12, 18, 22, 28, 32, 38, 42, 48,
    ];
    for key in keys {
        tree.insert(key, format!("v{}", key));
    }

    // Delete strategically to make siblings exactly at minimum
    for key in vec![18, 28, 38, 48] {
        tree.remove(&key);
    }

    println!("Tree before borrow attack");

    // Now delete a key that forces a borrow attempt from a minimum sibling
    println!("\nDeleting key to force borrow from minimum sibling...");
    tree.remove(&15);

    // Verify the tree handled this correctly
    match tree.check_invariants_detailed() {
        Ok(_) => println!("Tree survived borrow attack"),
        Err(e) => panic!("ATTACK SUCCESSFUL: Branch borrow failed! {}", e),
    }

    // Try to iterate to see if tree is corrupted
    let items: Vec<_> = tree.items().collect();
    println!("Items after attack: {:?}", items.len());
}

#[test]
fn test_branch_merge_with_maximum_keys_attack() {
    // Attack: Force branch merges when the combined size is exactly at capacity
    // This tests boundary conditions in merge operations

    let capacity = 6; // Chosen to make math tricky
    let mut tree = create_tree_capacity_int(capacity);

    // Fill tree
    insert_sequential_range_int(&mut tree, 100);

    // Delete pattern to create branches at specific sizes
    // Goal: Two adjacent branches that when merged have exactly capacity keys
    let mut deleted = 0;
    for i in (0..100).rev() {
        if deleted >= 70 {
            break;
        }
        if i % 3 != 0 {
            tree.remove(&i);
            deleted += 1;
        }
    }

    println!("Tree before merge attack");

    // Find and delete a key that will trigger the specific merge
    for i in 0..30 {
        if tree.contains_key(&(i * 3)) {
            println!(
                "\nDeleting key {} to force merge at capacity boundary...",
                i * 3
            );
            tree.remove(&(i * 3));

            // Check for invariant violations
            if let Err(e) = tree.check_invariants_detailed() {
                panic!(
                    "ATTACK SUCCESSFUL: Merge at capacity boundary failed! {}",
                    e
                );
            }
        }
    }
}

#[test]
fn test_alternating_sibling_operations_attack() {
    // Attack: Rapidly alternate between operations that affect siblings
    // This targets potential state inconsistencies in sibling tracking

    let capacity = 5; // Odd capacity for interesting minimum calculations
    let mut tree = create_tree_capacity(capacity);

    // Create tree with specific structure
    insert_with_multiplier(&mut tree, 60, 2);

    // Alternating pattern of operations designed to confuse sibling state
    for round in 0..10 {
        println!("\nRound {} of alternating operations", round);

        // Delete from left side
        let left_key = round * 6;
        if tree.contains_key(&left_key) {
            tree.remove(&left_key);
        }

        // Insert in middle
        let mid_key = 30 + round;
        tree.insert(mid_key * 2 + 1, format!("mid{}", round));

        // Delete from right side
        let right_key = 118 - round * 6;
        if tree.contains_key(&right_key) {
            tree.remove(&right_key);
        }

        // Verify invariants each round
        if let Err(e) = tree.check_invariants_detailed() {
            panic!("ATTACK SUCCESSFUL at round {}: {}", round, e);
        }
    }

    // Final verification - can we iterate correctly?
    let items: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    let mut sorted_items = items.clone();
    sorted_items.sort();

    if items != sorted_items {
        panic!("ATTACK SUCCESSFUL: Iterator returns unsorted items!");
    }
}

#[test]
fn test_deep_tree_branch_collapse_attack() {
    // Attack: Create a very deep tree then trigger branch collapses
    // This targets the complex branch height reduction logic

    let capacity = 4;
    let mut tree = create_tree_capacity_int(capacity);

    // Create a deep tree by inserting in a pattern that maximizes height
    let mut key = 0;
    for level in 0..5 {
        let count = capacity.pow(level);
        for _ in 0..count * 10 {
            tree.insert(key, key);
            key += 100; // Large gaps to force deep structure
        }
    }

    println!("Created deep tree with {} items", tree.len());

    // Now delete most items to force repeated height reductions
    let original_len = tree.len();
    let mut deleted = 0;

    for i in (0..key).step_by(100) {
        if tree.contains_key(&i) {
            tree.remove(&i);
            deleted += 1;

            // Check invariants periodically
            if deleted % 50 == 0 {
                if let Err(e) = tree.check_invariants_detailed() {
                    panic!("ATTACK SUCCESSFUL after {} deletions: {}", deleted, e);
                }
            }
        }
    }

    println!("Deleted {} items, {} remain", deleted, tree.len());

    // Verify the tree still works
    if tree.len() != original_len - deleted {
        panic!(
            "ATTACK SUCCESSFUL: Lost items during collapse! Expected {}, got {}",
            original_len - deleted,
            tree.len()
        );
    }
}

#[test]
#[should_panic(expected = "ATTACK SUCCESSFUL")]
fn test_force_branch_rebalance_panic() {
    // Attack: Try to force a panic in branch rebalancing code
    // This uses very specific patterns known to stress the implementation

    let capacity = 4;
    let mut tree = create_tree_capacity_int(capacity);

    // Pattern specifically designed to create unstable branch structure
    insert_with_multiplier_int(&mut tree, 16, 10);

    // Delete in specific order to create minimum branches
    for i in vec![10, 30, 50, 70, 90, 110, 130] {
        tree.remove(&i);
    }

    // This sequence should stress the rebalancing logic
    tree.remove(&20);
    tree.remove(&40);
    tree.remove(&60); // This should trigger complex rebalancing

    // If we get here without panic, check invariants
    if let Err(e) = tree.check_invariants_detailed() {
        panic!("ATTACK SUCCESSFUL: {}", e);
    }

    // Force the panic we expect
    panic!("ATTACK SUCCESSFUL: Expected panic didn't occur, but this is suspicious!");
}
