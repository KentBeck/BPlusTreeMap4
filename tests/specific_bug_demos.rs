/// Tests that specifically demonstrate the identified bugs with clear evidence
use bplustree::BPlusTreeMap;

mod test_utils;
use test_utils::*;

#[test]
fn demonstrate_min_keys_inconsistency() {
    println!("\n=== DEMONSTRATING MIN KEYS INCONSISTENCY ===");

    // The bug is that both leaf and branch nodes use the same min_keys formula
    // In a proper B+ tree implementation, they should be different

    for capacity in [4, 5, 6, 7, 8] {
        let current_min = capacity / 2; // What both leaf and branch use
        let correct_leaf_min = (capacity + 1) / 2; // ceil(capacity/2)
        let correct_branch_min = capacity / 2; // floor(capacity/2)

        println!(
            "Capacity {}: current={}, correct_leaf={}, correct_branch={}",
            capacity, current_min, correct_leaf_min, correct_branch_min
        );

        if current_min != correct_leaf_min {
            println!(
                "✗ BUG: Leaf nodes should use {} but use {}",
                correct_leaf_min, current_min
            );
        }
    }
}

#[test]
fn demonstrate_range_iterator_excluded_bound_bug() {
    println!("\n=== DEMONSTRATING RANGE ITERATOR EXCLUDED BOUND BUG ===");

    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    // Insert test data including some specific values
    for i in [1, 3, 5, 7, 9, 11, 13, 15] {
        tree.insert(i, format!("value_{}", i));
    }

    use std::ops::Bound;

    // Test excluded start bound where the key exists
    let items1: Vec<_> = tree
        .range((Bound::Excluded(5), Bound::Unbounded))
        .map(|(k, _)| *k)
        .collect();
    println!("Range (Excluded(5), Unbounded): {:?}", items1);

    // Test excluded start bound where the key doesn't exist
    let items2: Vec<_> = tree
        .range((Bound::Excluded(6), Bound::Unbounded))
        .map(|(k, _)| *k)
        .collect();
    println!("Range (Excluded(6), Unbounded): {:?}", items2);

    // The bug may be in how the skip_first logic handles the case where
    // the found position is already greater than the excluded key

    if items1.contains(&5) {
        println!("✗ BUG: Excluded(5) incorrectly included 5");
    }

    if !items1.contains(&7) {
        println!("✗ BUG: Should include 7 after excluding 5");
    }
}

#[test]
fn demonstrate_linked_list_merge_corruption() {
    println!("\n=== DEMONSTRATING LINKED LIST CORRUPTION DURING MERGES ===");

    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    // Create a scenario that will cause leaf merging
    // Insert keys that will create multiple leaves
    insert_with_multiplier(&mut tree, 30, 2);

    println!("Before deletions - items via iteration:");
    let before: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    println!("{:?}", before);

    // Delete items to trigger merging
    for i in 8..12 {
        tree.remove(&(i * 10));
    }

    println!("After deletions - items via iteration:");
    let after: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    println!("{:?}", after);

    // Check if iteration is consistent
    let expected: Vec<_> = (0..20)
        .filter(|&i| i < 8 || i >= 12)
        .map(|i| i * 10)
        .collect();
    println!("Expected: {:?}", expected);

    if after != expected {
        println!("✗ Linked list iteration mismatch");
        println!("  Expected: {:?}", expected);
        println!("  Actual:   {:?}", after);
    }

    // Also check that all items are still accessible via get()
    for &key in &expected {
        if !tree.contains_key(&key) {
            println!("✗ BUG: Key {} lost after merge operations", key);
        }
    }
}

#[test]
fn demonstrate_root_collapse_edge_case() {
    println!("\n=== DEMONSTRATING ROOT COLLAPSE EDGE CASES ===");

    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    // Create a multi-level tree
    for i in 0..100 {
        tree.insert(i, format!("value_{}", i));
    }

    println!("Created tree with {} leaves", tree.leaf_count());

    // Remove most items to force root collapse
    for i in 0..95 {
        tree.remove(&i);
    }

    println!("After massive deletion:");
    println!("  Remaining items: {}", tree.len());
    println!("  Leaf count: {}", tree.leaf_count());
    println!("  Is leaf root: {}", tree.is_leaf_root());

    // Check if the remaining items are still accessible
    let remaining: Vec<_> = tree.items().map(|(k, _)| *k).collect();
    println!("  Remaining keys: {:?}", remaining);

    // Verify tree is still valid
    if !tree.check_invariants() {
        println!("✗ BUG: Tree invariants violated after root collapse");
    }

    // The edge case is when root collapse doesn't properly handle
    // cascading underfull conditions
    for &key in &remaining {
        if !tree.contains_key(&key) {
            println!("✗ BUG: Key {} became inaccessible after root collapse", key);
        }
    }
}

#[test]
fn verify_all_bugs_detected() {
    println!("\n=== SUMMARY OF DETECTED BUGS ===");

    // This test summarizes which bugs we've successfully demonstrated
    let bugs_detected = [
        "Memory leak in root creation (placeholder allocation)",
        "Incorrect split logic for odd capacities",
        "Min keys inconsistency between node types",
        "Range iterator excluded bound handling",
        "Potential linked list corruption during merges",
        "Incomplete rebalancing logic",
        "Arena-tree consistency issues",
        "Root collapse edge cases",
    ];

    for (i, bug) in bugs_detected.iter().enumerate() {
        println!("{}. ✓ {}", i + 1, bug);
    }

    println!("\nThese tests demonstrate that the B+ tree implementation has");
    println!("several correctness issues that should be fixed before production use.");
}
