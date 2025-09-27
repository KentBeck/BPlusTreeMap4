use bplustree::BPlusTreeMap;

mod test_utils;
use test_utils::*;

#[test]
fn test_basic_deletion() {
    let mut tree = create_tree_capacity_int(4);
    tree.insert(42, 420);
    assert_eq!(tree.get(&42), Some(&420));
    assert_eq!(tree.remove(&42), Some(420));
    assert_eq!(tree.get(&42), None);
}
