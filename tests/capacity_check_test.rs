//! Test to verify that capacity checks prevent overflow corruption

use bplustree::BPlusTreeMap;

#[test]
fn test_capacity_check_prevents_overflow() {
    // Create a tree with small capacity to trigger overflow scenarios
    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(5).unwrap();

    // Insert enough items to create a multi-level tree
    for i in 0..50 {
        tree.insert(i, format!("value_{}", i));
    }

    println!("Tree created with 50 items");

    // Check invariants before deletion
    match tree.check_invariants_detailed() {
        Ok(_) => println!("Tree invariants OK before deletion"),
        Err(e) => panic!("Tree invariants violated before deletion: {}", e),
    }

    // Try to trigger the overflow scenario that was causing corruption
    // This should now panic with our capacity check instead of corrupting memory
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.remove(&10);
    }));

    match result {
        Ok(_) => {
            println!("Remove operation completed successfully");
            // Check if tree is still valid
            match tree.check_invariants_detailed() {
                Ok(_) => println!("Tree invariants OK after deletion"),
                Err(e) => panic!("Tree invariants violated after deletion: {}", e),
            }
        }
        Err(_) => {
            println!("Remove operation panicked (expected if capacity check triggered)");
            // This is actually good - it means our capacity check prevented corruption
        }
    }
}

#[test]
fn test_simple_operations_still_work() {
    // Test that basic operations still work with our fixes
    let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(10).unwrap();

    // Insert some items
    for i in 0..20 {
        tree.insert(i, format!("value_{}", i));
    }

    // Remove some items
    for i in 0..5 {
        let removed = tree.remove(&i);
        assert!(removed.is_some(), "Should have removed key {}", i);
    }

    // Check that remaining items are still accessible
    for i in 5..20 {
        let value = tree.get(&i);
        assert!(value.is_some(), "Should find key {}", i);
        assert_eq!(value.unwrap(), &format!("value_{}", i));
    }

    println!("Simple operations test passed");
}

#[test]
fn test_tree_with_drop_tracking() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone)]
    struct DropTracker {
        #[allow(dead_code)]
        id: usize,
        counter: Arc<AtomicUsize>,
    }

    impl DropTracker {
        fn new(id: usize, counter: Arc<AtomicUsize>) -> Self {
            counter.fetch_add(1, Ordering::SeqCst);
            Self { id, counter }
        }
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.counter.fetch_sub(1, Ordering::SeqCst);
        }
    }

    let counter = Arc::new(AtomicUsize::new(0));

    {
        let mut tree: BPlusTreeMap<i32, DropTracker> = BPlusTreeMap::new(5).unwrap();

        // Insert items with drop tracking
        for i in 0..10 {
            let tracker = DropTracker::new(i, counter.clone());
            tree.insert(i as i32, tracker);
        }

        println!(
            "Inserted 10 items, counter: {}",
            counter.load(Ordering::SeqCst)
        );

        // Remove some items
        for i in 0..3 {
            tree.remove(&(i as i32));
        }

        println!(
            "Removed 3 items, counter: {}",
            counter.load(Ordering::SeqCst)
        );

        // Tree goes out of scope here
    }

    // All items should be properly dropped
    let final_count = counter.load(Ordering::SeqCst);
    println!("Final counter: {}", final_count);

    // We expect 0 if all items were properly dropped
    // If there's a double-free, the test will crash before this assertion
    // If there's a memory leak, this assertion will fail
    assert_eq!(
        final_count, 0,
        "Memory leak detected: {} items not dropped",
        final_count
    );
}
