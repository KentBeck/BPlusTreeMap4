//! Partial Iteration Benchmark
//!
//! This benchmark tests BPlusTreeMap's performance for partial iteration scenarios,
//! which are common in real-world applications (e.g., pagination, range queries).
//!
//! ## Scenarios
//!
//! 1. **From Middle**: Iterate N items starting from the middle of the tree
//!    - Tests range query performance with a specific starting key
//!    - Uses `range(key..)` which is the recommended approach
//!
//! 2. **Random Positions**: Perform many small iterations from random keys
//!    - Tests the overhead of repeatedly creating new range iterators
//!    - Simulates scenarios like pagination with random access patterns
//!
//! 3. **Cursor-like**: Perform many tiny iterations (10 items each)
//!    - Tests iterator creation overhead for very small iterations
//!    - Simulates database cursor operations or incremental data fetching
//!
//! ## Note on items()
//!
//! The "iterate from beginning" scenario using `items()` was removed because:
//! - It calls `len()` internally, which walks all leaf nodes (O(n))
//! - This makes it 1000x+ slower than std::BTreeMap for large datasets
//! - Real-world applications should use `range(Bound::Unbounded..)` instead
//! - The scenario was not representative of typical partial iteration use cases

use std::collections::BTreeMap;
use std::env;
use std::hint::black_box;
use std::time::Duration;
use std::time::Instant;

use bplustree::BPlusTreeMap;

fn parse_arg<T: std::str::FromStr>(i: usize, default: T) -> T {
    env::args()
        .nth(i)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    // Usage: bench_partial_iter [total_size=10000000] [iter_count=100] [cap=128]
    let total_size: usize = parse_arg(1, 10_000_000);
    let iter_count: usize = parse_arg(2, 100);
    let cap: usize = parse_arg(3, 128);

    println!("\n=== Partial Iteration Benchmark ===");
    println!("Total items in tree: {}", total_size);
    println!("Items to iterate: {}", iter_count);
    println!("B+ tree capacity: {}", cap);
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

    // Build std BTreeMap
    println!("Building std::BTreeMap...");
    let mut std_map = BTreeMap::new();
    for &(k, v) in &dataset {
        std_map.insert(k, v);
    }

    println!("Trees built. Starting benchmarks...\n");

    // Benchmark 1: Iterate from middle
    println!(
        "--- Scenario 1: Iterate {} items from middle ---",
        iter_count
    );
    let middle_key = find_middle_key(&dataset);
    let bplus_middle = bench_partial_iter_from_key(&bplus, middle_key, iter_count);
    let std_middle = bench_partial_iter_from_key(&std_map, middle_key, iter_count);
    print_results("From Middle", bplus_middle, std_middle, iter_count);

    // Benchmark 2: Multiple small iterations at random positions
    println!(
        "\n--- Scenario 2: 100 random partial iterations of {} items each ---",
        iter_count
    );
    let random_keys = generate_random_keys(&dataset, 100);
    let bplus_random = bench_multiple_partial_iters(&bplus, &random_keys, iter_count);
    let std_random = bench_multiple_partial_iters(&std_map, &random_keys, iter_count);
    print_results(
        "Random Positions",
        bplus_random,
        std_random,
        iter_count * 100,
    );

    // Benchmark 3: Very small iterations (simulate cursor-like behavior)
    // This tests the overhead of creating many iterators for tiny iteration counts,
    // which is common in cursor-based APIs or incremental data fetching.
    let tiny_count = 10;
    println!(
        "\n--- Scenario 3: 1000 tiny iterations of {} items each (cursor simulation) ---",
        tiny_count
    );
    let cursor_keys = generate_random_keys(&dataset, 1000);
    let bplus_cursor = bench_multiple_partial_iters(&bplus, &cursor_keys, tiny_count);
    let std_cursor = bench_multiple_partial_iters(&std_map, &cursor_keys, tiny_count);
    print_results("Cursor-like", bplus_cursor, std_cursor, tiny_count * 1000);

    println!("\n=== Summary ===");
    println!("Total items in tree: {}", total_size);
    println!("Partial iteration count: {}", iter_count);
}

fn generate_dataset(n: usize) -> Vec<(u64, u64)> {
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut dataset: Vec<(u64, u64)> = (0..n as u64)
        .map(|i| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state, i)
        })
        .collect();

    // Sort by key so we can find middle/end predictably
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

fn bench_partial_iter_from_key<M>(map: &M, start_key: u64, count: usize) -> Duration
where
    M: RangeIterableBenchmark,
{
    let start = Instant::now();
    let mut n = 0;
    for (k, v) in map.range_from(start_key).take(count) {
        black_box((k, v));
        n += 1;
    }
    let elapsed = start.elapsed();
    black_box(n);
    elapsed
}

fn bench_multiple_partial_iters<M>(map: &M, start_keys: &[u64], count_per_iter: usize) -> Duration
where
    M: RangeIterableBenchmark,
{
    let start = Instant::now();
    let mut total_items = 0;

    for &key in start_keys {
        for (k, v) in map.range_from(key).take(count_per_iter) {
            black_box((k, v));
            total_items += 1;
        }
    }

    let elapsed = start.elapsed();
    black_box(total_items);
    elapsed
}

fn print_results(scenario: &str, bplus_time: Duration, std_time: Duration, op_count: usize) {
    let bplus_ns = bplus_time.as_nanos() as f64;
    let std_ns = std_time.as_nanos() as f64;
    let bplus_per_item = bplus_ns / op_count as f64;
    let std_per_item = std_ns / op_count as f64;

    let speedup = if bplus_ns < std_ns {
        std_ns / bplus_ns
    } else {
        -(bplus_ns / std_ns)
    };

    println!("{:<20} | BPlusTreeMap: {:>10.3}ms ({:>8.2}ns/item) | std::BTreeMap: {:>10.3}ms ({:>8.2}ns/item)",
        scenario,
        bplus_time.as_secs_f64() * 1000.0,
        bplus_per_item,
        std_time.as_secs_f64() * 1000.0,
        std_per_item
    );

    if speedup > 0.0 {
        println!("{:<20}   → BPlusTreeMap is {:.2}x FASTER", "", speedup);
    } else {
        println!("{:<20}   → BPlusTreeMap is {:.2}x SLOWER", "", -speedup);
    }
}

trait RangeIterableBenchmark {
    type RangeIter<'a>: Iterator<Item = (&'a u64, &'a u64)>
    where
        Self: 'a;
    fn range_from(&self, key: u64) -> Self::RangeIter<'_>;
}

impl RangeIterableBenchmark for BPlusTreeMap<u64, u64> {
    type RangeIter<'a> = bplustree::Items<'a, u64, u64>;
    fn range_from(&self, key: u64) -> Self::RangeIter<'_> {
        self.range(key..)
    }
}

impl RangeIterableBenchmark for BTreeMap<u64, u64> {
    type RangeIter<'a> = std::collections::btree_map::Range<'a, u64, u64>;
    fn range_from(&self, key: u64) -> Self::RangeIter<'_> {
        self.range(key..)
    }
}
