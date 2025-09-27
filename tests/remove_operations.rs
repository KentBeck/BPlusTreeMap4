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

#[test]
fn test_delete_nonexistent_key() {
    let mut tree = create_tree_capacity_int(4);
    tree.insert(42, 420);
    assert_eq!(tree.remove(&42), Some(420));
    assert_eq!(tree.remove(&42), None);
}

#[test]
fn test_delete_from_branch_tree() {
    let mut tree = create_tree_capacity_int(4);
    for i in 0..8 {
        tree.insert(i, i * 10);
    }
    assert!(!tree.is_leaf_root());
    assert_eq!(tree.remove(&3), Some(30));
    assert_eq!(tree.get(&3), None);
    for i in 0..8 {
        if i != 3 {
            assert_eq!(tree.get(&i), Some(&(i * 10)));
        }
    }
}

#[test]
fn test_delete_forces_root_collapse() {
    let mut tree = create_tree_capacity_int(4);
    for i in 0..5 {
        tree.insert(i, i * 10);
    }
    assert!(!tree.is_leaf_root());
    assert_eq!(tree.remove(&0), Some(0));
    assert_eq!(tree.get(&0), None);
    for i in 1..5 {
        assert_eq!(tree.get(&i), Some(&(i * 10)));
    }
    assert!(tree.is_leaf_root());
}
