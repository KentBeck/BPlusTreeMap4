//! Focused benchmark for profiling insert/delete operations
//! This benchmark is designed to make memmove operations visible in profiling tools

use bplustree::BPlusTreeMap;
use std::hint::black_box;

fn main() {
    let n = 1_000_000;
    let capacity = 128;

    println!("=== Insert/Delete Profiling Benchmark ===");
    println!("Items: {}", n);
    println!("Capacity: {}", capacity);
    println!();

    // Generate dataset
    println!("Generating dataset...");
    let mut keys: Vec<u64> = (0..n).map(|i| i as u64 * 7919).collect();

    // Randomize to avoid best-case scenarios
    let mut state: u64 = 0x9E3779B97F4A7C15;
    for i in 0..keys.len() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state as usize) % keys.len();
        keys.swap(i, j);
    }

    println!("Starting insert workload (hot path)...");

    // Workload 1: Sequential inserts (many memmove operations)
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for &key in &keys {
        black_box(tree.insert(key, key));
    }
    println!("Inserted {} items", n);

    // Workload 2: Interleaved insert/delete (stress memmove)
    println!("Starting insert/delete churn workload...");
    for round in 0..5 {
        println!("  Round {}/5", round + 1);

        // Delete every other item
        let mut deleted = 0;
        for i in (0..n).step_by(2) {
            if tree.remove(&keys[i]).is_some() {
                deleted += 1;
            }
        }
        black_box(deleted);

        // Re-insert them
        let mut inserted = 0;
        for i in (0..n).step_by(2) {
            tree.insert(keys[i], keys[i]);
            inserted += 1;
        }
        black_box(inserted);
    }

    // Workload 3: Many small deletes (middle of nodes)
    println!("Starting targeted delete workload...");
    let delete_count = n / 4;
    let mut deleted_total = 0;
    for i in 0..delete_count {
        if tree.remove(&keys[i]).is_some() {
            deleted_total += 1;
        }
    }
    println!("Deleted {} items", deleted_total);

    // Workload 4: Refill with inserts
    println!("Starting refill workload...");
    let mut inserted_total = 0;
    for i in 0..delete_count {
        tree.insert(keys[i], keys[i]);
        inserted_total += 1;
    }
    println!("Re-inserted {} items", inserted_total);

    // Workload 5: Delete everything in random order
    println!("Starting full deletion workload...");
    let mut delete_order: Vec<usize> = (0..n).collect();
    let mut state: u64 = 0xFEDCBA9876543210;
    for i in 0..delete_order.len() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state as usize) % delete_order.len();
        delete_order.swap(i, j);
    }

    let mut final_deleted = 0;
    for &idx in &delete_order {
        if tree.remove(&keys[idx]).is_some() {
            final_deleted += 1;
        }
    }
    println!("Final deletion: {} items", final_deleted);

    println!("\nProfiling workload complete!");
    println!("Total operations performed:");
    println!("  Initial inserts: {}", n);
    println!("  Churn rounds: 5 (delete+insert cycles)");
    println!(
        "  Targeted operations: {} deletes + {} inserts",
        delete_count, delete_count
    );
    println!("  Final deletion: {}", n);
}
