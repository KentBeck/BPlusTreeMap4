use bplustree::BPlusTreeMap;

#[test]
fn test_range_syntax_inclusive() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Test inclusive range 3..=7
    let range: Vec<_> = tree.range(3..=7).map(|(k, v)| (*k, v.clone())).collect();
    assert_eq!(
        range,
        vec![
            (3, "value3".to_string()),
            (4, "value4".to_string()),
            (5, "value5".to_string()),
            (6, "value6".to_string()),
            (7, "value7".to_string()),
        ]
    );
}

#[test]
fn test_range_syntax_exclusive() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Test exclusive range 3..7
    let range: Vec<_> = tree.range(3..7).map(|(k, v)| (*k, v.clone())).collect();
    assert_eq!(
        range,
        vec![
            (3, "value3".to_string()),
            (4, "value4".to_string()),
            (5, "value5".to_string()),
            (6, "value6".to_string()),
        ]
    );
}

#[test]
fn test_range_syntax_from() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Test from range 5..
    let range: Vec<_> = tree.range(5..).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![5, 6, 7, 8, 9]);
}

#[test]
fn test_range_syntax_to() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Test to range ..5
    let range: Vec<_> = tree.range(..5).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_range_syntax_to_inclusive() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Test to inclusive range ..=5
    let range: Vec<_> = tree.range(..=5).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn test_range_syntax_full() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Test full range ..
    let range: Vec<_> = tree.range(..).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_range_syntax_empty_ranges() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Empty range - start > end
    let range: Vec<_> = tree.range(7..3).collect();
    assert_eq!(range, vec![]);

    // Empty range - out of bounds
    let range: Vec<_> = tree.range(100..200).collect();
    assert_eq!(range, vec![]);

    // Empty range - exclusive same value
    let range: Vec<_> = tree.range(5..5).collect();
    assert_eq!(range, vec![]);
}

#[test]
fn test_range_syntax_edge_cases() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i * 2, format!("value{}", i * 2)); // Even numbers only
    }

    // Range with non-existent bounds
    let range: Vec<_> = tree.range(3..=7).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![4, 6]); // Only even numbers in range

    // Exclusive start that doesn't exist
    let range: Vec<_> = tree.range(3..8).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![4, 6]);

    // Inclusive end that doesn't exist
    let range: Vec<_> = tree.range(4..=7).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![4, 6]);
}

#[test]
fn test_range_syntax_with_strings() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    let keys = vec!["apple", "banana", "cherry", "date", "elderberry", "fig"];
    for key in &keys {
        tree.insert(key.to_string(), format!("{}_value", key));
    }

    // String range inclusive
    let range: Vec<_> = tree
        .range("banana".to_string()..="date".to_string())
        .map(|(k, _)| k.clone())
        .collect();
    assert_eq!(range, vec!["banana", "cherry", "date"]);

    // String range exclusive
    let range: Vec<_> = tree
        .range("banana".to_string().."elderberry".to_string())
        .map(|(k, _)| k.clone())
        .collect();
    assert_eq!(range, vec!["banana", "cherry", "date"]);
}

#[test]
fn test_range_syntax_single_element() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Single element with inclusive range
    let range: Vec<_> = tree.range(5..=5).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![5]);

    // Single element with exclusive end (should be empty)
    let range: Vec<_> = tree.range(5..6).map(|(k, _)| *k).collect();
    assert_eq!(range, vec![5]);
}

#[test]
fn test_range_syntax_excluded_start() {
    let mut tree = BPlusTreeMap::new(16).unwrap();
    for i in 0..10 {
        tree.insert(i, format!("value{}", i));
    }

    // Using (Bound::Excluded, Bound::Included) via a custom range type
    use std::ops::{Bound, RangeBounds};

    struct ExcludedStart {
        start: i32,
        end: i32,
    }

    impl RangeBounds<i32> for ExcludedStart {
        fn start_bound(&self) -> Bound<&i32> {
            Bound::Excluded(&self.start)
        }

        fn end_bound(&self) -> Bound<&i32> {
            Bound::Included(&self.end)
        }
    }

    let range = ExcludedStart { start: 3, end: 6 };
    let result: Vec<_> = tree.range(range).map(|(k, _)| *k).collect();
    assert_eq!(result, vec![4, 5, 6]); // 3 is excluded
}
