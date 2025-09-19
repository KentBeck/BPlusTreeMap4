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
fn test_leaf_sizes() {
    println!("Creating tree...");
    let tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(4).unwrap();

    println!("Getting leaf sizes...");
    let sizes = tree.leaf_sizes();
    println!("Leaf sizes: {:?}", sizes);

    assert_eq!(sizes, vec![0]); // Empty tree should have 1 leaf with 0 keys
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

    let sizes = tree.leaf_sizes();
    println!("Leaf sizes after 20 insertions: {:?}", sizes);

    // Check the distribution - it should be reasonably balanced
    let min_size = *sizes.iter().min().unwrap();
    let max_size = *sizes.iter().max().unwrap();

    println!("Min leaf size: {}, Max leaf size: {}", min_size, max_size);

    // The difference shouldn't be too large
    assert!(
        max_size - min_size <= 2,
        "Leaf sizes too unbalanced: {:?}",
        sizes
    );
}
