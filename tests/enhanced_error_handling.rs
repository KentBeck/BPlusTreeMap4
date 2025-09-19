//! Enhanced error handling tests
//! These tests verify the improved error handling patterns, Result type aliases,
//! and convenience methods for robust B+ tree operations

use bplustree::{
    BPlusTreeError, BPlusTreeMap, BTreeResult, BTreeResultExt, InitResult, KeyResult, ModifyResult,
};

mod test_utils;

use test_utils::*;

// ============================================================================
// ERROR CONSTRUCTION AND FORMATTING TESTS
// ============================================================================

#[test]
fn test_enhanced_error_constructors() {
    println!("=== ENHANCED ERROR CONSTRUCTORS TEST ===");

    // Test InvalidCapacity with context
    let error = BPlusTreeError::invalid_capacity(2, 4);
    assert!(error.to_string().contains("Capacity 2 is invalid"));
    assert!(error.to_string().contains("minimum required: 4"));

    // Test DataIntegrityError with context
    let error = BPlusTreeError::data_integrity("Split operation", "Key collision detected");
    assert!(error.to_string().contains("Split operation"));
    assert!(error.to_string().contains("Key collision detected"));

    // Test ArenaError with context
    let error = BPlusTreeError::arena_error("Node allocation", "Out of memory");
    assert!(error.to_string().contains("Node allocation failed"));
    assert!(error.to_string().contains("Out of memory"));

    // Test NodeError with context
    let error = BPlusTreeError::node_error("Leaf", 42, "Corruption detected");
    assert!(error.to_string().contains("Leaf node 42"));
    assert!(error.to_string().contains("Corruption detected"));

    // Test CorruptedTree with context
    let error = BPlusTreeError::corrupted_tree("Linked list", "Cycle detected");
    assert!(error.to_string().contains("Linked list corruption"));
    assert!(error.to_string().contains("Cycle detected"));

    // Test InvalidState with context
    let error = BPlusTreeError::invalid_state("insert", "tree is locked");
    assert!(error.to_string().contains("Cannot insert"));
    assert!(error.to_string().contains("tree is locked"));

    // Test AllocationError with context
    let error = BPlusTreeError::allocation_error("leaf node", "arena full");
    assert!(error.to_string().contains("Failed to allocate leaf node"));
    assert!(error.to_string().contains("arena full"));

    println!("✅ Enhanced error constructors working correctly");
}

// ============================================================================
// RESULT TYPE ALIASES TESTS
// ============================================================================

#[test]
fn test_result_type_aliases() {
    println!("=== RESULT TYPE ALIASES TEST ===");

    // Test InitResult
    let init_result: InitResult<BPlusTreeMap<i32, String>> = BPlusTreeMap::new(4);
    assert!(init_result.is_ok());

    let invalid_init: InitResult<BPlusTreeMap<i32, String>> = BPlusTreeMap::new(2);
    assert!(invalid_init.is_err());

    // Test KeyResult
    let tree = create_tree_4_with_data(10);
    let key_result: KeyResult<&String> = tree.get_item(&5);
    assert!(key_result.is_ok());

    let missing_key: KeyResult<&String> = tree.get_item(&999);
    assert!(missing_key.is_err());

    // Test ModifyResult
    let mut tree = create_tree_4();
    let modify_result: ModifyResult<String> = tree.remove_item(&999);
    assert!(modify_result.is_err());

    // Test BTreeResult for general operations
    let general_result: BTreeResult<()> = tree.validate_for_operation("test");
    assert!(general_result.is_ok());

    println!("✅ Result type aliases working correctly");
}

// ============================================================================
// RESULT EXTENSION TRAIT TESTS
// ============================================================================

#[test]
fn test_result_extension_trait() {
    println!("=== RESULT EXTENSION TRAIT TEST ===");

    let tree = create_tree_4_with_data(5);

    // Test with_context
    let result: KeyResult<&String> = tree.get_item(&999);
    let with_context = result.with_context("User lookup operation");
    assert!(with_context.is_err());
    assert!(with_context
        .unwrap_err()
        .to_string()
        .contains("Key not found"));

    // Test with_operation
    let result: KeyResult<&String> = tree.get_item(&888);
    let with_operation = result.with_operation("find_user");
    assert!(with_operation.is_err());
    assert!(with_operation
        .unwrap_err()
        .to_string()
        .contains("Key not found"));

    // Test or_default_with_log for types that implement Default
    let result: Result<Vec<String>, BPlusTreeError> = Err(BPlusTreeError::KeyNotFound);
    let default_value = result.or_default_with_log();
    assert_eq!(default_value, Vec::<String>::new());

    println!("✅ Result extension trait working correctly");
}

// ============================================================================
// CONVENIENCE METHODS TESTS
// ============================================================================

#[test]
fn test_get_or_default() {
    println!("=== GET OR DEFAULT TEST ===");

    let tree = create_tree_4_with_data(5);
    let default_value = "default".to_string();

    // Test existing key
    let value = tree.get_or_default(&2, &default_value);
    assert_eq!(value, &"value_2".to_string());

    // Test missing key
    let value = tree.get_or_default(&999, &default_value);
    assert_eq!(value, &default_value);

    println!("✅ get_or_default working correctly");
}

#[test]
fn test_try_get() {
    println!("=== TRY GET TEST ===");

    let tree = create_tree_4_with_data(5);

    // Test existing key
    let result = tree.try_get(&2);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), &"value_2".to_string());

    // Test missing key with context
    let result = tree.try_get(&999);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Key not found"));

    println!("✅ try_get working correctly");
}

#[test]
fn test_try_insert_and_try_remove() {
    println!("=== TRY INSERT AND TRY REMOVE TEST ===");

    let mut tree = create_tree_4();

    // Test try_insert
    let result = tree.try_insert(1, "value_1".to_string());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);

    // Test try_insert with existing key
    let result = tree.try_insert(1, "new_value_1".to_string());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("value_1".to_string()));

    // Test try_remove
    let result = tree.try_remove(&1);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "new_value_1".to_string());

    // Test try_remove with missing key
    let result = tree.try_remove(&999);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Key not found"));

    println!("✅ try_insert and try_remove working correctly");
}

#[test]
fn test_batch_insert() {
    println!("=== BATCH INSERT TEST ===");

    let mut tree = create_tree_4();

    // Test successful batch insert
    let items = vec![
        (1, "value_1".to_string()),
        (2, "value_2".to_string()),
        (3, "value_3".to_string()),
    ];

    let result = tree.batch_insert(items);
    assert!(result.is_ok());
    let old_values = result.unwrap();
    assert_eq!(old_values, vec![None, None, None]);

    // Verify all items were inserted
    assert_eq!(tree.len(), 3);
    assert_eq!(tree.get(&1), Some(&"value_1".to_string()));
    assert_eq!(tree.get(&2), Some(&"value_2".to_string()));
    assert_eq!(tree.get(&3), Some(&"value_3".to_string()));

    println!("✅ batch_insert working correctly");
}

#[test]
fn test_get_many() {
    println!("=== GET MANY TEST ===");

    let tree = create_tree_4_with_data(10);

    // Test successful get_many
    let keys = [1, 3, 5, 7];
    let result = tree.get_many(&keys);
    assert!(result.is_ok());
    let values = result.unwrap();
    assert_eq!(values.len(), 4);
    assert_eq!(values[0], &"value_1".to_string());
    assert_eq!(values[1], &"value_3".to_string());
    assert_eq!(values[2], &"value_5".to_string());
    assert_eq!(values[3], &"value_7".to_string());

    // Test get_many with missing key
    let keys = [1, 999, 3];
    let result = tree.get_many(&keys);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Key not found"));

    println!("✅ get_many working correctly");
}

#[test]
fn test_validate_for_operation() {
    println!("=== VALIDATE FOR OPERATION TEST ===");

    let tree = create_tree_4_with_data(5);

    // Test validation on valid tree
    let result = tree.validate_for_operation("user_lookup");
    assert!(result.is_ok());

    println!("✅ validate_for_operation working correctly");
}

// ============================================================================
// ERROR CONTEXT PROPAGATION TESTS
// ============================================================================

#[test]
fn test_error_context_propagation() {
    println!("=== ERROR CONTEXT PROPAGATION TEST ===");

    let tree = create_tree_4_with_data(5);

    // Test that error context is properly propagated through the chain
    let result = tree
        .get_item(&999)
        .with_context("Database lookup")
        .with_operation("find_user_by_id");

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Key not found"));

    println!("✅ Error context propagation working correctly");
}

// ============================================================================
// INTEGRATION TESTS WITH EXISTING API
// ============================================================================

#[test]
fn test_integration_with_existing_api() {
    println!("=== INTEGRATION WITH EXISTING API TEST ===");

    let mut tree = create_tree_4();

    // Mix old and new API methods
    tree.insert(1, "old_api".to_string());

    let result = tree.try_insert(2, "new_api".to_string());
    assert!(result.is_ok());

    // Use old get with new error handling
    let value = tree
        .get(&1)
        .ok_or(BPlusTreeError::KeyNotFound)
        .with_context("Mixed API usage");
    assert!(value.is_ok());

    // Verify both methods work together
    assert_eq!(tree.len(), 2);
    assert_invariants(&tree, "mixed API integration");

    println!("✅ Integration with existing API working correctly");
}

// ============================================================================
// ERROR RECOVERY TESTS
// ============================================================================

#[test]
fn test_error_recovery_patterns() {
    println!("=== ERROR RECOVERY PATTERNS TEST ===");

    let tree = create_tree_4_with_data(5);

    // Test graceful degradation with get_or_default
    let fallback = "fallback_value".to_string();
    let value = tree.get_or_default(&999, &fallback);
    assert_eq!(value, &fallback);

    // Test error logging with or_default_with_log
    let result: Result<Vec<String>, BPlusTreeError> = Err(BPlusTreeError::KeyNotFound);
    let default_vec = result.or_default_with_log();
    assert!(default_vec.is_empty());

    println!("✅ Error recovery patterns working correctly");
}

// ============================================================================
// PERFORMANCE AND MEMORY TESTS
// ============================================================================

#[test]
fn test_error_handling_performance() {
    println!("=== ERROR HANDLING PERFORMANCE TEST ===");

    let tree = create_tree_4_with_data(1000);

    // Test that error handling doesn't significantly impact performance
    let start = std::time::Instant::now();

    for i in 0..100 {
        let _ = tree.try_get(&i);
    }

    let duration = start.elapsed();
    println!("100 try_get operations took: {:?}", duration);

    // Should complete quickly (exact time depends on system, but should be < 1ms)
    assert!(
        duration.as_millis() < 10,
        "Error handling operations too slow"
    );

    println!("✅ Error handling performance acceptable");
}

#[cfg(test)]
mod comprehensive_tests {
    use super::*;

    #[test]
    fn test_comprehensive_error_scenario() {
        println!("=== COMPREHENSIVE ERROR SCENARIO TEST ===");

        // Create a tree and perform various operations that could fail
        let mut tree = create_tree_4();

        // Test the full error handling pipeline
        let batch_items = vec![
            (1, "item_1".to_string()),
            (2, "item_2".to_string()),
            (3, "item_3".to_string()),
        ];

        // Batch insert with validation
        tree.validate_for_operation("batch_insert").unwrap();
        let result = tree.batch_insert(batch_items);
        assert!(result.is_ok());

        // Multi-key lookup with error context
        let keys = [1, 2, 3];
        let values = tree
            .get_many(&keys)
            .with_context("User profile lookup")
            .with_operation("load_user_profiles");
        assert!(values.is_ok());

        // Try operations with validation
        let new_value = tree
            .try_insert(4, "item_4".to_string())
            .with_context("Adding new user");
        assert!(new_value.is_ok());

        let removed_value = tree.try_remove(&1).with_context("Deleting user");
        assert!(removed_value.is_ok());

        // Final validation
        tree.validate_for_operation("final_check").unwrap();
        assert_invariants(&tree, "comprehensive error scenario");

        println!("✅ Comprehensive error scenario completed successfully");
    }
}
