//! Fuzz tests for BPlusTree
//!
//! These tests are marked with `#[ignore]` so they don't run during normal `cargo test`.
//!
//! To run fuzz tests:
//! - All fuzz tests: `cargo test --test fuzz_tests -- --ignored`
//! - Specific test: `cargo test fuzz_test_bplustree -- --ignored --nocapture`
//! - With custom timing: `FUZZ_TIME=30s cargo test fuzz_test_timed -- --ignored --nocapture`

use bplustree::BPlusTreeMap;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::time::{Duration, Instant};

#[test]
#[ignore]
fn fuzz_test_bplustree() {
    // Test with various branching factors (minimum 4 required)
    for branching_factor in 4..=10 {
        println!("\n=== Testing branching factor {} ===", branching_factor);

        let mut bplustree = BPlusTreeMap::new(branching_factor).unwrap();
        let mut btree_map = BTreeMap::new();
        let mut operations = Vec::new();

        // Insert keys until we have up to 20 leaf nodes
        let mut key = 1;
        let mut iteration = 0;

        while bplustree.leaf_count() < 20 && iteration < 1000 {
            let value = key * 10;

            // Record the operation
            operations.push(format!("insert({}, {})", key, value));

            // Insert into both trees
            let bplus_result = bplustree.insert(key, value);
            let btree_result = btree_map.insert(key, value);

            // Check that insert results match
            if bplus_result != btree_result {
                println!("MISMATCH on insert({}, {}):", key, value);
                println!("BPlusTree returned: {:?}", bplus_result);
                println!("BTreeMap returned: {:?}", btree_result);
                println!("Operations so far:");
                for op in &operations {
                    println!("  {}", op);
                }
                panic!("Insert result mismatch!");
            }

            // Verify all previously inserted keys can still be found
            for check_key in 1..=key {
                let bplus_value = bplustree.get(&check_key);
                let btree_value = btree_map.get(&check_key);

                if bplus_value != btree_value {
                    println!(
                        "MISMATCH on get({}) after insert({}, {}):",
                        check_key, key, value
                    );
                    println!("BPlusTree returned: {:?}", bplus_value);
                    println!("BTreeMap returned: {:?}", btree_value);
                    println!("BPlusTree has {} nodes", bplustree.leaf_count(),);
                    println!("Operations so far:");
                    for op in &operations {
                        println!("  {}", op);
                    }
                    println!("Tree structure:");
                    panic!("Get result mismatch!");
                }
            }

            // Verify tree length matches
            if bplustree.len() != btree_map.len() {
                println!("LENGTH MISMATCH after insert({}, {}):", key, value);
                println!("BPlusTree len: {}", bplustree.len());
                println!("BTreeMap len: {}", btree_map.len());
                println!("Operations so far:");
                for op in &operations {
                    println!("  {}", op);
                }
                panic!("Length mismatch!");
            }

            // Verify iteration order matches
            let bplus_slice: Vec<_> = bplustree.items().collect();
            let btree_slice: Vec<_> = btree_map.iter().collect();

            if bplus_slice.len() != btree_slice.len() {
                println!(
                    "ITERATION LENGTH MISMATCH after insert({}, {}):",
                    key, value
                );
                println!("BPlusTree iteration len: {}", bplus_slice.len());
                println!("BTreeMap slice len: {}", btree_slice.len());
                println!("Operations so far:");
                for op in &operations {
                    println!("  {}", op);
                }
                panic!("Slice length mismatch!");
            }

            for (i, (bplus_item, btree_item)) in
                bplus_slice.iter().zip(btree_slice.iter()).enumerate()
            {
                if bplus_item.0 != btree_item.0 || bplus_item.1 != btree_item.1 {
                    println!(
                        "SLICE ORDER MISMATCH at index {} after insert({}, {}):",
                        i, key, value
                    );
                    println!("BPlusTree item: ({:?}, {:?})", bplus_item.0, bplus_item.1);
                    println!("BTreeMap item: ({:?}, {:?})", btree_item.0, btree_item.1);
                    println!("BPlusTree slice: {:?}", bplus_slice);
                    println!("BTreeMap slice: {:?}", btree_slice);
                    println!("Operations so far:");
                    for op in &operations {
                        println!("  {}", op);
                    }
                    panic!("Slice order mismatch!");
                }
            }

            key += 1;
            iteration += 1;

            // Print progress every 10 insertions
            if key % 10 == 0 {
                println!(
                    "  Inserted {} keys, {} nodes",
                    key - 1,
                    bplustree.leaf_count(),
                );
            }
        }

        println!(
            "Successfully tested branching factor {} with {} keys and {} leaf nodes",
            branching_factor,
            key - 1,
            bplustree.leaf_count()
        );
    }
}

#[test]
#[ignore]
fn fuzz_test_with_random_keys() {
    // Test with random insertion order
    for branching_factor in [4, 5, 8] {
        println!(
            "\n=== Testing branching factor {} with random keys ===",
            branching_factor
        );

        let mut bplustree = BPlusTreeMap::new(branching_factor).unwrap();
        let mut btree_map = BTreeMap::new();
        let mut operations = Vec::new();
        let mut inserted_keys = HashSet::new();

        // Generate a set of keys to insert
        let mut keys_to_insert = Vec::new();
        for i in 1..=100 {
            keys_to_insert.push(i);
        }

        // Insert keys in a specific "random" pattern (deterministic for reproducibility)
        let pattern = [3, 7, 1, 9, 5, 2, 8, 4, 6, 0]; // Cycle through this pattern
        let mut key_index = 0;

        while bplustree.leaf_count() < 15 && key_index < keys_to_insert.len() {
            // Pick key using the pattern
            let pattern_index = key_index % pattern.len();
            let offset = pattern[pattern_index];
            let actual_key_index = (key_index + offset * 7) % keys_to_insert.len();
            let key = keys_to_insert[actual_key_index];

            // Skip if already inserted
            if inserted_keys.contains(&key) {
                key_index += 1;
                continue;
            }

            let value = key * 10;
            inserted_keys.insert(key);

            // Record the operation
            operations.push(format!("insert({}, {})", key, value));

            // Insert into both trees
            let bplus_result = bplustree.insert(key, value);
            let btree_result = btree_map.insert(key, value);

            // Check that insert results match
            if bplus_result != btree_result {
                println!("MISMATCH on insert({}, {}):", key, value);
                println!("BPlusTree returned: {:?}", bplus_result);
                println!("BTreeMap returned: {:?}", btree_result);
                println!("Operations so far:");
                for op in &operations {
                    println!("  {}", op);
                }
                panic!("Insert result mismatch!");
            }

            // Verify all previously inserted keys can still be found
            for &check_key in &inserted_keys {
                let bplus_value = bplustree.get(&check_key);
                let btree_value = btree_map.get(&check_key);

                if bplus_value != btree_value {
                    println!(
                        "MISMATCH on get({}) after insert({}, {}):",
                        check_key, key, value
                    );
                    println!("BPlusTree returned: {:?}", bplus_value);
                    println!("BTreeMap returned: {:?}", btree_value);
                    println!("BPlusTree has {} nodes", bplustree.leaf_count(),);
                    println!("Operations so far:");
                    for op in &operations {
                        println!("  {}", op);
                    }
                    println!("Tree structure:");
                    panic!("Get result mismatch!");
                }
            }

            key_index += 1;

            // Print progress every 20 insertions
            if inserted_keys.len() % 20 == 0 {
                println!(
                    "  Inserted {} keys, {} nodes",
                    inserted_keys.len(),
                    bplustree.leaf_count(),
                );
            }
        }

        println!(
            "Successfully tested branching factor {} with {} random keys and {} leaf nodes",
            branching_factor,
            inserted_keys.len(),
            bplustree.leaf_count()
        );
    }
}

#[test]
#[ignore]
fn fuzz_test_with_updates() {
    // Test updating existing keys
    for branching_factor in [4, 7] {
        println!(
            "\n=== Testing branching factor {} with updates ===",
            branching_factor
        );

        let mut bplustree = BPlusTreeMap::new(branching_factor).unwrap();
        let mut btree_map = BTreeMap::new();
        let mut operations = Vec::new();

        // First insert some keys
        for key in 1..=50 {
            let value = key * 10;
            operations.push(format!("insert({}, {})", key, value));
            bplustree.insert(key, value);
            btree_map.insert(key, value);
        }

        // Now update some keys
        let update_keys = [5, 15, 25, 35, 45, 1, 50, 20, 30, 40];
        for &key in &update_keys {
            let new_value = key * 100;
            operations.push(format!("update({}, {})", key, new_value));

            let bplus_result = bplustree.insert(key, new_value);
            let btree_result = btree_map.insert(key, new_value);

            // Check that update results match (should return old value)
            if bplus_result != btree_result {
                println!("MISMATCH on update({}, {}):", key, new_value);
                println!("BPlusTree returned: {:?}", bplus_result);
                println!("BTreeMap returned: {:?}", btree_result);
                println!("Operations so far:");
                for op in &operations {
                    println!("  {}", op);
                }
                panic!("Update result mismatch!");
            }

            // Verify the new value is retrievable
            let bplus_value = bplustree.get(&key);
            let btree_value = btree_map.get(&key);

            if bplus_value != btree_value {
                println!("MISMATCH on get({}) after update:", key);
                println!("BPlusTree returned: {:?}", bplus_value);
                println!("BTreeMap returned: {:?}", btree_value);
                println!("Operations so far:");
                for op in &operations {
                    println!("  {}", op);
                }
                panic!("Get after update mismatch!");
            }
        }

        println!(
            "Successfully tested updates with branching factor {}",
            branching_factor
        );
    }
}

/// Timed fuzz test that runs for a specified duration.
///
/// Usage:
/// - Default (10 seconds): `cargo test fuzz_test_timed -- --ignored --nocapture`
/// - Custom duration: `FUZZ_TIME=30s cargo test fuzz_test_timed -- --ignored --nocapture`
/// - Minutes: `FUZZ_TIME=5m cargo test fuzz_test_timed -- --ignored --nocapture`
/// - Hours: `FUZZ_TIME=1h cargo test fuzz_test_timed -- --ignored --nocapture`
/// - Milliseconds: `FUZZ_TIME=500ms cargo test fuzz_test_timed -- --ignored --nocapture`
#[test]
#[ignore]
fn fuzz_test_timed() {
    // Parse time duration from environment variable or default to 10 seconds
    let duration_str = env::var("FUZZ_TIME").unwrap_or_else(|_| "10s".to_string());
    let duration = parse_duration(&duration_str).unwrap_or(Duration::from_secs(10));

    println!("Running timed fuzz test for {:?}", duration);

    let start_time = Instant::now();
    let mut total_operations = 0;
    let mut total_keys_inserted = 0;
    let mut max_nodes_reached = 0;

    while start_time.elapsed() < duration {
        // Cycle through different branching factors
        for branching_factor in [4, 5, 7, 8, 10] {
            if start_time.elapsed() >= duration {
                break;
            }

            let mut bplustree = BPlusTreeMap::new(branching_factor).unwrap();
            let mut btree_map = BTreeMap::new();
            let mut operations = Vec::new();

            // Run until we hit time limit or reach a reasonable number of nodes
            let mut key = 1;
            while start_time.elapsed() < duration && bplustree.leaf_count() < 50 {
                let value = key * 10;

                // Record the operation
                operations.push(format!("insert({}, {})", key, value));
                total_operations += 1;

                // Insert into both trees
                let bplus_result = bplustree.insert(key, value);
                let btree_result = btree_map.insert(key, value);

                // Check that insert results match
                if bplus_result != btree_result {
                    println!(
                        "MISMATCH on insert({}, {}) with branching factor {}:",
                        key, value, branching_factor
                    );
                    println!("BPlusTree returned: {:?}", bplus_result);
                    println!("BTreeMap returned: {:?}", btree_result);
                    println!("Recent operations:");
                    for op in operations.iter().rev().take(10) {
                        println!("  {}", op);
                    }
                    panic!("Insert result mismatch!");
                }

                // Periodically verify all keys can be found
                if key % 10 == 0 {
                    for check_key in 1..=key {
                        let bplus_value = bplustree.get(&check_key);
                        let btree_value = btree_map.get(&check_key);

                        if bplus_value != btree_value {
                            println!(
                                "MISMATCH on get({}) with branching factor {}:",
                                check_key, branching_factor
                            );
                            println!("BPlusTree returned: {:?}", bplus_value);
                            println!("BTreeMap returned: {:?}", btree_value);
                            println!("Tree has {} nodes ", bplustree.leaf_count(),);
                            println!("Recent operations:");
                            for op in operations.iter().rev().take(20) {
                                println!("  {}", op);
                            }
                            panic!("Get result mismatch!");
                        }
                    }
                }

                key += 1;
                total_keys_inserted += 1;
                max_nodes_reached = max_nodes_reached.max(bplustree.leaf_count());
            }
        }
    }

    println!("Timed fuzz test completed successfully!");
    println!("Duration: {:?}", start_time.elapsed());
    println!("Total operations: {}", total_operations);
    println!("Total keys inserted: {}", total_keys_inserted);
    println!("Max nodes reached: {}", max_nodes_reached);
}

// Helper function to parse duration strings like "10s", "5m", "1h"
fn parse_duration(s: &str) -> Result<Duration, String> {
    if s.is_empty() {
        return Err("Empty duration string".to_string());
    }

    let (number_part, unit_part) = if let Some(pos) = s.chars().position(|c| c.is_alphabetic()) {
        (&s[..pos], &s[pos..])
    } else {
        return Err("No unit found in duration string".to_string());
    };

    let number: u64 = number_part
        .parse()
        .map_err(|_| format!("Invalid number: {}", number_part))?;

    let duration = match unit_part {
        "s" | "sec" | "seconds" => Duration::from_secs(number),
        "m" | "min" | "minutes" => Duration::from_secs(number * 60),
        "h" | "hour" | "hours" => Duration::from_secs(number * 3600),
        "ms" | "milliseconds" => Duration::from_millis(number),
        _ => return Err(format!("Unknown time unit: {}", unit_part)),
    };

    Ok(duration)
}
