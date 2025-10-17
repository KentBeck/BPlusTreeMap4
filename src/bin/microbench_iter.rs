//! Micro-benchmark to measure individual operation costs in partial iteration
//!
//! This benchmark isolates specific operations to identify hotspots:
//! 1. Iterator creation overhead
//! 2. leaf_for_key cost
//! 3. carve_leaf cost
//! 4. Binary search cost
//! 5. Per-item iteration cost
//! 6. Leaf traversal cost

use bplustree::BPlusTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let tree_size = 10_000_000;
    let capacity = 128;
    let iterations = 10_000;

    println!("=== BPlusTreeMap Partial Iteration Micro-Benchmark ===");
    println!("Tree size: {}", tree_size);
    println!("Capacity: {}", capacity);
    println!("Iterations: {}", iterations);
    println!();

    // Build tree
    println!("Building tree...");
    let dataset = generate_dataset(tree_size);
    let mut tree = BPlusTreeMap::new(capacity).expect("new tree");
    for &(k, v) in &dataset {
        tree.insert(k, v);
    }
    println!("Tree built.\n");

    // Test 1: Iterator creation cost
    println!("=== Test 1: Iterator Creation Overhead ===");
    let test_keys = generate_random_keys(&dataset, iterations);

    let start = Instant::now();
    for &key in &test_keys {
        let iter = black_box(tree.range(key..));
        black_box(iter);
    }
    let elapsed = start.elapsed();
    let per_create = elapsed.as_nanos() as f64 / iterations as f64;
    println!(
        "Create {} iterators: {:.3}ms",
        iterations,
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Per iterator creation: {:.2}ns", per_create);
    println!();

    // Test 2: Iterator creation + first next() call
    println!("=== Test 2: Iterator Creation + First next() ===");
    let start = Instant::now();
    for &key in &test_keys {
        let mut iter = tree.range(key..);
        black_box(iter.next());
    }
    let elapsed = start.elapsed();
    let per_first = elapsed.as_nanos() as f64 / iterations as f64;
    println!(
        "Create + first next: {:.3}ms",
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Per operation: {:.2}ns", per_first);
    println!("First next() overhead: {:.2}ns", per_first - per_create);
    println!();

    // Test 3: Small iteration (10 items)
    println!("=== Test 3: Small Iteration (10 items) ===");
    let start = Instant::now();
    let mut total = 0;
    for &key in &test_keys {
        for (k, v) in tree.range(key..).take(10) {
            black_box((k, v));
            total += 1;
        }
    }
    let elapsed = start.elapsed();
    let per_item_10 = elapsed.as_nanos() as f64 / total as f64;
    println!(
        "Iterate {} x 10 items = {}: {:.3}ms",
        iterations,
        total,
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Per item: {:.2}ns", per_item_10);
    println!();

    // Test 4: Medium iteration (100 items)
    println!("=== Test 4: Medium Iteration (100 items) ===");
    let start = Instant::now();
    let mut total = 0;
    for &key in test_keys.iter().take(1000) {
        for (k, v) in tree.range(key..).take(100) {
            black_box((k, v));
            total += 1;
        }
    }
    let elapsed = start.elapsed();
    let per_item_100 = elapsed.as_nanos() as f64 / total as f64;
    println!(
        "Iterate 1000 x 100 items = {}: {:.3}ms",
        total,
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Per item: {:.2}ns", per_item_100);
    println!();

    // Test 5: Within-leaf iteration (no leaf boundary crossing)
    println!("=== Test 5: Within-Leaf Iteration (no boundary crossing) ===");
    let middle_key = dataset[dataset.len() / 2].0;
    let start = Instant::now();
    let mut total = 0;
    for _ in 0..iterations {
        for (k, v) in tree.range(middle_key..).take(50) {
            black_box((k, v));
            total += 1;
        }
    }
    let elapsed = start.elapsed();
    let per_item_same = elapsed.as_nanos() as f64 / total as f64;
    println!(
        "Iterate {} x 50 items = {}: {:.3}ms",
        iterations,
        total,
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Per item (mostly same leaf): {:.2}ns", per_item_same);
    println!();

    // Test 6: Cross-leaf iteration (force leaf boundary crossing)
    println!("=== Test 6: Cross-Leaf Iteration (force boundary crossing) ===");
    let start = Instant::now();
    let mut total = 0;
    for &key in test_keys.iter().take(500) {
        for (k, v) in tree.range(key..).take(200) {
            black_box((k, v));
            total += 1;
        }
    }
    let elapsed = start.elapsed();
    let per_item_cross = elapsed.as_nanos() as f64 / total as f64;
    println!(
        "Iterate 500 x 200 items = {}: {:.3}ms",
        total,
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Per item (with leaf crossing): {:.2}ns", per_item_cross);
    println!();

    // Test 7: Bounded range queries
    println!("=== Test 7: Bounded Range Queries ===");
    let key_pairs = generate_key_pairs(&dataset, 1000);
    let start = Instant::now();
    let mut total = 0;
    for (start_key, end_key) in &key_pairs {
        for (k, v) in tree.range(*start_key..*end_key) {
            black_box((k, v));
            total += 1;
        }
    }
    let elapsed = start.elapsed();
    let per_item_bounded = elapsed.as_nanos() as f64 / total as f64;
    println!(
        "1000 bounded ranges, {} items: {:.3}ms",
        total,
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Per item: {:.2}ns", per_item_bounded);
    println!();

    // Summary
    println!("=== Summary ===");
    println!("Iterator creation only:        {:.2}ns", per_create);
    println!("Iterator + first next():       {:.2}ns", per_first);
    println!("Per-item (10 item batches):    {:.2}ns", per_item_10);
    println!("Per-item (100 item batches):   {:.2}ns", per_item_100);
    println!("Per-item (within leaf):        {:.2}ns", per_item_same);
    println!("Per-item (cross leaf):         {:.2}ns", per_item_cross);
    println!("Per-item (bounded range):      {:.2}ns", per_item_bounded);
    println!();
    println!("=== Analysis ===");
    println!("Setup overhead per iteration:  {:.2}ns", per_first);
    println!(
        "Amortized item cost (small):   {:.2}ns",
        per_item_10 - (per_first / 10.0)
    );
    println!(
        "Amortized item cost (medium):  {:.2}ns",
        per_item_100 - (per_first / 100.0)
    );
    println!(
        "Leaf boundary crossing cost:   ~{:.2}ns",
        per_item_cross - per_item_same
    );
}

fn generate_dataset(n: usize) -> Vec<(u64, u64)> {
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut dataset: Vec<(u64, u64)> = (0..n as u64)
        .map(|i| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state, i)
        })
        .collect();
    dataset.sort_by_key(|(k, _)| *k);
    dataset
}

fn generate_random_keys(dataset: &[(u64, u64)], count: usize) -> Vec<u64> {
    let mut state: u64 = 0x123456789ABCDEF0;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state as usize) % dataset.len();
        keys.push(dataset[idx].0);
    }
    keys
}

fn generate_key_pairs(dataset: &[(u64, u64)], count: usize) -> Vec<(u64, u64)> {
    let mut state: u64 = 0xFEDCBA9876543210;
    let mut pairs = Vec::with_capacity(count);
    for _ in 0..count {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx1 = (state as usize) % (dataset.len() - 200);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let span = 50 + ((state as usize) % 150);
        let idx2 = idx1 + span;
        pairs.push((dataset[idx1].0, dataset[idx2].0));
    }
    pairs
}
