//! Micro-benchmark to measure memmove overhead in insert/delete operations
//!
//! This benchmark measures how much time is spent in memory copy operations
//! versus other overhead (tree traversal, allocation, etc.)

use bplustree::BPlusTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let n = 500_000;
    let capacity = 128;

    println!("=== Memmove Overhead Analysis ===");
    println!("Items: {}", n);
    println!("Capacity: {}", capacity);
    println!();

    // Generate dataset
    let mut keys: Vec<u64> = (0..n).map(|i| i as u64 * 7919).collect();

    // Randomize to create realistic conditions
    let mut state: u64 = 0x9E3779B97F4A7C15;
    for i in 0..keys.len() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state as usize) % keys.len();
        keys.swap(i, j);
    }

    println!("=== Part 1: Insert Operations ===\n");

    // Measure insert time
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    let start = Instant::now();
    for &key in &keys {
        black_box(tree.insert(key, key));
    }
    let insert_time = start.elapsed();

    println!("Total insert time: {:.3}s", insert_time.as_secs_f64());
    println!(
        "Per-insert: {:.2}ns",
        insert_time.as_nanos() as f64 / n as f64
    );
    println!();

    // Estimate memmove overhead for inserts
    // Assumptions based on B+ tree characteristics:
    // - Capacity 128 means ~64 items per node on average when balanced
    // - Insert in middle of node requires shifting ~32 items on average
    // - Each shift moves 16 bytes (u64 key + u64 value)
    // - Memory bandwidth: ~50 GB/s for sequential writes (realistic for DDR4)

    let items_per_node_avg = capacity / 2;
    let items_to_shift_avg = items_per_node_avg / 2;
    let bytes_per_item = 16; // u64 key + u64 value
    let bytes_per_shift = items_to_shift_avg * bytes_per_item;
    let memory_bandwidth_gbs = 50.0; // Conservative estimate
    let memory_bandwidth_bps = memory_bandwidth_gbs * 1_000_000_000.0;

    let estimated_memmove_per_insert =
        (bytes_per_shift as f64) / memory_bandwidth_bps * 1_000_000_000.0;
    let total_estimated_memmove = estimated_memmove_per_insert * n as f64;

    println!("Estimated memmove overhead (theoretical):");
    println!("  Avg items to shift per insert: {}", items_to_shift_avg);
    println!("  Avg bytes to move per insert: {}", bytes_per_shift);
    println!("  Time per memmove: {:.2}ns", estimated_memmove_per_insert);
    println!(
        "  Total memmove time: {:.3}s",
        total_estimated_memmove / 1_000_000_000.0
    );
    println!(
        "  Memmove percentage: {:.1}%",
        (total_estimated_memmove / insert_time.as_nanos() as f64) * 100.0
    );
    println!();

    println!("=== Part 2: Delete Operations ===\n");

    // Measure delete time
    let start = Instant::now();
    for &key in &keys {
        black_box(tree.remove(&key));
    }
    let delete_time = start.elapsed();

    println!("Total delete time: {:.3}s", delete_time.as_secs_f64());
    println!(
        "Per-delete: {:.2}ns",
        delete_time.as_nanos() as f64 / n as f64
    );
    println!();

    // Estimate memmove overhead for deletes
    // Similar to inserts, deletes shift remaining items
    println!("Estimated memmove overhead (theoretical):");
    println!("  Avg items to shift per delete: {}", items_to_shift_avg);
    println!("  Avg bytes to move per delete: {}", bytes_per_shift);
    println!("  Time per memmove: {:.2}ns", estimated_memmove_per_insert);
    println!(
        "  Total memmove time: {:.3}s",
        total_estimated_memmove / 1_000_000_000.0
    );
    println!(
        "  Memmove percentage: {:.1}%",
        (total_estimated_memmove / delete_time.as_nanos() as f64) * 100.0
    );
    println!();

    println!("=== Part 3: Mixed Workload (Insert then Delete) ===\n");

    // Rebuild tree
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for &key in &keys {
        tree.insert(key, key);
    }

    // Now do interleaved operations
    let ops = 100_000;
    let start = Instant::now();
    for i in 0..ops {
        let idx = i % keys.len();
        tree.remove(&keys[idx]);
        tree.insert(keys[idx], keys[idx]);
    }
    let mixed_time = start.elapsed();

    println!("Mixed operations: {} delete+insert pairs", ops);
    println!("Total time: {:.3}s", mixed_time.as_secs_f64());
    println!(
        "Per pair: {:.2}ns",
        mixed_time.as_nanos() as f64 / ops as f64
    );
    println!();

    let estimated_memmove_mixed = (estimated_memmove_per_insert * 2.0) * ops as f64;
    println!("Estimated memmove overhead:");
    println!(
        "  Total memmove time: {:.3}s",
        estimated_memmove_mixed / 1_000_000_000.0
    );
    println!(
        "  Memmove percentage: {:.1}%",
        (estimated_memmove_mixed / mixed_time.as_nanos() as f64) * 100.0
    );
    println!();

    println!("=== Summary ===\n");
    println!("Operation         Total Time    Per-Op (ns)   Est. Memmove %");
    println!("─────────────────────────────────────────────────────────────");
    println!(
        "Insert            {:>8.3}s    {:>10.2}    {:>7.1}%",
        insert_time.as_secs_f64(),
        insert_time.as_nanos() as f64 / n as f64,
        (total_estimated_memmove / insert_time.as_nanos() as f64) * 100.0
    );
    println!(
        "Delete            {:>8.3}s    {:>10.2}    {:>7.1}%",
        delete_time.as_secs_f64(),
        delete_time.as_nanos() as f64 / n as f64,
        (total_estimated_memmove / delete_time.as_nanos() as f64) * 100.0
    );
    println!(
        "Mixed (del+ins)   {:>8.3}s    {:>10.2}    {:>7.1}%",
        mixed_time.as_secs_f64(),
        mixed_time.as_nanos() as f64 / ops as f64,
        (estimated_memmove_mixed / mixed_time.as_nanos() as f64) * 100.0
    );
    println!();

    println!("=== Analysis ===\n");
    println!("Theoretical estimates assume:");
    println!("  - Average node fill: 50% (64 items for capacity 128)");
    println!("  - Average shift distance: 25% of node (32 items)");
    println!("  - Memory bandwidth: {} GB/s", memory_bandwidth_gbs);
    println!("  - Sequential memory operations");
    println!();

    let insert_non_memmove =
        100.0 - (total_estimated_memmove / insert_time.as_nanos() as f64) * 100.0;
    let delete_non_memmove =
        100.0 - (total_estimated_memmove / delete_time.as_nanos() as f64) * 100.0;

    println!("Non-memmove overhead (tree traversal, allocation, bookkeeping):");
    println!("  Insert: {:.1}%", insert_non_memmove);
    println!("  Delete: {:.1}%", delete_non_memmove);
    println!();

    if insert_non_memmove > 50.0 {
        println!("⚠️  Inserts: High non-memmove overhead suggests room for optimization");
    } else if insert_non_memmove < 30.0 {
        println!("✅ Inserts: Low overhead - well-optimized, mostly memory-bound");
    } else {
        println!("✓  Inserts: Reasonable overhead - good balance");
    }

    if delete_non_memmove > 50.0 {
        println!("⚠️  Deletes: High non-memmove overhead suggests room for optimization");
    } else if delete_non_memmove < 30.0 {
        println!("✅ Deletes: Low overhead - well-optimized, mostly memory-bound");
    } else {
        println!("✓  Deletes: Reasonable overhead - good balance");
    }
    println!();

    println!("Note: These are theoretical estimates. Actual memmove time depends on:");
    println!("  - CPU memory bandwidth and latency");
    println!("  - Cache hit rates");
    println!("  - Memory alignment");
    println!("  - Node split/merge patterns");
    println!("  - Actual shift distances (not always average)");
}
