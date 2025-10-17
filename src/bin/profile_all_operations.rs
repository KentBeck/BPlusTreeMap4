//! Comprehensive profiling benchmark for all BPlusTreeMap operations
//!
//! This benchmark measures detailed performance breakdown for:
//! 1. GET operations
//! 2. INSERT operations
//! 3. DELETE operations
//! 4. PARTIAL ITERATION operations

use bplustree::BPlusTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let n = 1_000_000;
    let capacity = 128;

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║     COMPREHENSIVE OPERATION PROFILING - BPlusTreeMap         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Configuration:");
    println!("  Items: {}", n);
    println!("  Capacity: {}", capacity);
    println!();

    // Generate dataset
    println!("Generating dataset...");
    let mut keys: Vec<u64> = (0..n).map(|i| i as u64 * 7919).collect();

    // Randomize for realistic distribution
    let mut state: u64 = 0x9E3779B97F4A7C15;
    for i in 0..keys.len() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state as usize) % keys.len();
        keys.swap(i, j);
    }

    // Build initial tree
    println!("Building initial tree...");
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for &key in &keys {
        tree.insert(key, key);
    }
    println!("Tree built with {} items\n", n);

    // ═══════════════════════════════════════════════════════════════
    // OPERATION 1: GET
    // ═══════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("OPERATION 1: GET (Lookup)");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    profile_get(&tree, &keys);

    // ═══════════════════════════════════════════════════════════════
    // OPERATION 2: INSERT
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("OPERATION 2: INSERT");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    profile_insert(&keys, capacity);

    // ═══════════════════════════════════════════════════════════════
    // OPERATION 3: DELETE
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("OPERATION 3: DELETE (Remove)");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    profile_delete(&keys, capacity);

    // ═══════════════════════════════════════════════════════════════
    // OPERATION 4: PARTIAL ITERATION
    // ═══════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("OPERATION 4: PARTIAL ITERATION");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    profile_partial_iteration(&tree, &keys);

    // ═══════════════════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════════════════
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    PROFILING COMPLETE                         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
}

fn profile_get(tree: &BPlusTreeMap<u64, u64>, keys: &[u64]) {
    let test_count = 100_000;

    println!("Test 1: Sequential lookups (first {} items)", test_count);
    let start = Instant::now();
    for i in 0..test_count {
        let result = tree.get(&keys[i]);
        black_box(result);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 2: Random lookups ({} items)", test_count);
    let mut state: u64 = 0x123456789ABCDEF0;
    let start = Instant::now();
    for _ in 0..test_count {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state as usize) % keys.len();
        let result = tree.get(&keys[idx]);
        black_box(result);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 3: Cache-friendly clustered lookups");
    let cluster_size = 10;
    let num_clusters = test_count / cluster_size;
    let start = Instant::now();
    for _ in 0..num_clusters {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let base_idx = (state as usize) % (keys.len() - cluster_size);
        for offset in 0..cluster_size {
            let result = tree.get(&keys[base_idx + offset]);
            black_box(result);
        }
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 4: Missing key lookups");
    let start = Instant::now();
    for i in 0..test_count {
        let missing_key = keys[i] + 1; // Keys are spaced, so +1 should miss
        let result = tree.get(&missing_key);
        black_box(result);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("GET Summary:");
    println!("  Primary cost: Tree traversal + binary search in leaf");
    println!("  Expected bottlenecks: leaf_for_key, binary_search, cache misses");
}

fn profile_insert(keys: &[u64], capacity: usize) {
    let test_count = 100_000;

    println!("Test 1: Sequential inserts (empty tree)");
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    let start = Instant::now();
    for i in 0..test_count {
        tree.insert(keys[i], keys[i]);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 2: Random inserts (empty tree)");
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    let mut shuffled = keys[..test_count].to_vec();
    let mut state: u64 = 0x9E3779B97F4A7C15;
    for i in 0..shuffled.len() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state as usize) % shuffled.len();
        shuffled.swap(i, j);
    }
    let start = Instant::now();
    for &key in &shuffled {
        tree.insert(key, key);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 3: Updates (overwrite existing)");
    let start = Instant::now();
    for i in 0..test_count {
        tree.insert(shuffled[i], shuffled[i] * 2);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 4: Inserts causing splits (large tree)");
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for i in 0..500_000 {
        tree.insert(keys[i], keys[i]);
    }
    let insert_keys = &keys[500_000..500_000 + test_count];
    let start = Instant::now();
    for &key in insert_keys {
        tree.insert(key, key);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("INSERT Summary:");
    println!("  Primary costs: Tree traversal + memmove + node splits");
    println!("  Expected bottlenecks: leaf_for_key, shift_right, allocation");
}

fn profile_delete(keys: &[u64], capacity: usize) {
    let test_count = 100_000;

    println!("Test 1: Sequential deletes (front of dataset)");
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for &key in keys {
        tree.insert(key, key);
    }
    let start = Instant::now();
    for i in 0..test_count {
        tree.remove(&keys[i]);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 2: Random deletes");
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for &key in keys {
        tree.insert(key, key);
    }
    let mut state: u64 = 0x123456789ABCDEF0;
    let start = Instant::now();
    for _ in 0..test_count {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state as usize) % keys.len();
        tree.remove(&keys[idx]);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 3: Delete from middle of nodes");
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for &key in keys {
        tree.insert(key, key);
    }
    let middle_keys = &keys[keys.len() / 4..keys.len() / 4 + test_count];
    let start = Instant::now();
    for &key in middle_keys {
        tree.remove(&key);
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-op: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!();

    println!("Test 4: Delete causing merges (sparse tree)");
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for i in 0..test_count * 2 {
        tree.insert(keys[i], keys[i]);
    }
    // Delete every other item to make tree sparse
    for i in (0..test_count).step_by(2) {
        tree.remove(&keys[i]);
    }
    // Now delete remaining items (will cause merges)
    let start = Instant::now();
    for i in (1..test_count).step_by(2) {
        tree.remove(&keys[i]);
    }
    let elapsed = start.elapsed();
    let ops = test_count / 2;
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Per-op: {:.2}ns", elapsed.as_nanos() as f64 / ops as f64);
    println!();

    println!("DELETE Summary:");
    println!("  Primary costs: Tree traversal + binary search + memmove + merges");
    println!("  Expected bottlenecks: leaf_for_key, shift_left, rebalancing");
}

fn profile_partial_iteration(tree: &BPlusTreeMap<u64, u64>, keys: &[u64]) {
    let test_count = 10_000;

    println!("Test 1: Small iterations from random positions (10 items each)");
    let mut state: u64 = 0x123456789ABCDEF0;
    let start = Instant::now();
    let mut total_items = 0;
    for _ in 0..test_count {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state as usize) % keys.len();
        for (k, v) in tree.range(keys[idx]..).take(10) {
            black_box((k, v));
            total_items += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-iteration: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count as f64
    );
    println!(
        "  Per-item: {:.2}ns",
        elapsed.as_nanos() as f64 / total_items as f64
    );
    println!();

    println!("Test 2: Medium iterations from random positions (100 items each)");
    let test_count_medium = 1_000;
    let mut state: u64 = 0xFEDCBA9876543210;
    let start = Instant::now();
    let mut total_items = 0;
    for _ in 0..test_count_medium {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state as usize) % keys.len();
        for (k, v) in tree.range(keys[idx]..).take(100) {
            black_box((k, v));
            total_items += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-iteration: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count_medium as f64
    );
    println!(
        "  Per-item: {:.2}ns",
        elapsed.as_nanos() as f64 / total_items as f64
    );
    println!();

    println!("Test 3: Bounded range queries");
    let test_count_range = 1_000;
    let mut state: u64 = 0x555555555555555;
    let start = Instant::now();
    let mut total_items = 0;
    for _ in 0..test_count_range {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state as usize) % (keys.len() - 200);
        let start_key = keys[idx];
        let end_key = keys[idx + 100];
        for (k, v) in tree.range(start_key..end_key) {
            black_box((k, v));
            total_items += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-range: {:.2}ns",
        elapsed.as_nanos() as f64 / test_count_range as f64
    );
    println!(
        "  Per-item: {:.2}ns",
        elapsed.as_nanos() as f64 / total_items as f64
    );
    println!();

    println!("Test 4: Cursor-like iteration (many tiny iterations)");
    let cursor_count = 50_000;
    let items_per_cursor = 5;
    let mut state: u64 = 0xAAAAAAAAAAAAAAAA;
    let start = Instant::now();
    let mut total_items = 0;
    for _ in 0..cursor_count {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state as usize) % keys.len();
        for (k, v) in tree.range(keys[idx]..).take(items_per_cursor) {
            black_box((k, v));
            total_items += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Per-cursor: {:.2}ns",
        elapsed.as_nanos() as f64 / cursor_count as f64
    );
    println!(
        "  Per-item: {:.2}ns",
        elapsed.as_nanos() as f64 / total_items as f64
    );
    println!();

    println!("PARTIAL ITERATION Summary:");
    println!("  Primary costs: Iterator creation + tree traversal + item access");
    println!("  Expected bottlenecks: leaf_for_key (first next), bound checking");
}
