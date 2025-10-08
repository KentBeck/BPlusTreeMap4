/// Debug test to find the infinite loop
use bplustree::BPlusTreeMap;

mod test_utils;
use test_utils::*;

#[test]
fn test_empty_tree_leaf_count() {
    println!("Creating tree...");
    let tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    println!("Getting leaf count...");
    let count = tree.leaf_count();
    println!("Leaf count: {}", count);

    assert_eq!(count, 1); // Empty tree should have 1 leaf
}

#[test]
fn test_tree_creation_only() {
    println!("Creating tree...");
    let _tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();
    println!("Tree created successfully!");
}

#[test]
fn test_single_insertion() {
    println!("Creating tree...");
    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    println!("Inserting one item...");
    tree.insert(1, "one".to_string());

    println!("Getting leaf count...");
    let count = tree.leaf_count();
    println!("Leaf count: {}", count);

    assert_eq!(count, 1); // Should still have 1 leaf
}

#[test]
fn test_split_balance() {
    println!("Testing split balance with capacity 5...");
    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(5).unwrap();

    // Insert enough items to force splits and see the distribution
    insert_sequential_range(&mut tree, 20);

    // Verify tree maintains invariants after splits
    assert!(
        tree.check_invariants(),
        "Tree should maintain invariants after splits"
    );
}
