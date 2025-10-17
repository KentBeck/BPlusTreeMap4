//! Profiling-focused benchmark for BPlusTreeMap partial iteration
//! This runs ONLY BPlusTreeMap operations (no std::BTreeMap comparison)
//! to make profiling results clearer.

use bplustree::BPlusTreeMap;
use std::env;
use std::hint::black_box;

fn parse_arg<T: std::str::FromStr>(i: usize, default: T) -> T {
    env::args()
        .nth(i)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    // Usage: profile_partial_iter [total_size=10000000] [iter_count=100] [cap=128] [iterations=1000]
    let total_size: usize = parse_arg(1, 10_000_000);
    let iter_count: usize = parse_arg(2, 100);
    let cap: usize = parse_arg(3, 128);
    let num_iterations: usize = parse_arg(4, 1000);

    println!("=== BPlusTreeMap Partial Iteration Profiling ===");
    println!("Total items: {}", total_size);
    println!("Items per iteration: {}", iter_count);
    println!("Capacity: {}", cap);
    println!("Number of iterations: {}", num_iterations);
    println!();

    // Generate dataset
    println!("Generating dataset...");
    let dataset = generate_dataset(total_size);

    // Build B+ tree
    println!("Building BPlusTreeMap...");
    let mut bplus = BPlusTreeMap::new(cap).expect("new bplustree");
    for &(k, v) in &dataset {
        bplus.insert(k, v);
    }

    println!("Setup complete. Starting profiling workload...\n");

    // Workload 1: Range queries from random positions
    println!(
        "Workload 1: {} range queries from random positions",
        num_iterations
    );
    let random_keys = generate_random_keys(&dataset, num_iterations);
    let mut total_items = 0;
    for &key in &random_keys {
        for (k, v) in bplus.range(key..).take(iter_count) {
            black_box((k, v));
            total_items += 1;
        }
    }
    println!("  Processed {} items", total_items);

    // Workload 2: Small cursor-like iterations
    println!(
        "\nWorkload 2: {} cursor-like iterations (10 items each)",
        num_iterations * 5
    );
    let cursor_keys = generate_random_keys(&dataset, num_iterations * 5);
    total_items = 0;
    for &key in &cursor_keys {
        for (k, v) in bplus.range(key..).take(10) {
            black_box((k, v));
            total_items += 1;
        }
    }
    println!("  Processed {} items", total_items);

    // Workload 3: Sequential scans from middle
    println!(
        "\nWorkload 3: {} sequential scans from middle",
        num_iterations / 10
    );
    let middle_key = find_middle_key(&dataset);
    total_items = 0;
    for _ in 0..(num_iterations / 10) {
        for (k, v) in bplus.range(middle_key..).take(iter_count) {
            black_box((k, v));
            total_items += 1;
        }
    }
    println!("  Processed {} items", total_items);

    // Workload 4: Bounded range queries
    println!("\nWorkload 4: {} bounded range queries", num_iterations / 2);
    let range_keys = generate_key_pairs(&dataset, num_iterations / 2);
    total_items = 0;
    for (start_key, end_key) in range_keys {
        for (k, v) in bplus.range(start_key..end_key) {
            black_box((k, v));
            total_items += 1;
        }
    }
    println!("  Processed {} items", total_items);

    println!("\nProfiling workload complete!");
}

fn generate_dataset(n: usize) -> Vec<(u64, u64)> {
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut dataset: Vec<(u64, u64)> = (0..n as u64)
        .map(|i| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state, i)
        })
        .collect();

    // Sort by key
    dataset.sort_by_key(|(k, _)| *k);
    dataset
}

fn find_middle_key(dataset: &[(u64, u64)]) -> u64 {
    dataset[dataset.len() / 2].0
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
        let idx1 = (state as usize) % (dataset.len() - 100);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let span = 50 + ((state as usize) % 200); // 50-250 items
        let idx2 = idx1 + span;
        pairs.push((dataset[idx1].0, dataset[idx2.min(dataset.len() - 1)].0));
    }
    pairs
}
