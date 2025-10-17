use bplustree::BPlusTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let n = 10_000_000;
    let cap = 128;
    let iter_count = 100;

    println!("Building tree with {} items (capacity {})...", n, cap);

    // Generate sorted dataset
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut dataset: Vec<(u64, u64)> = (0..n as u64)
        .map(|i| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state, i)
        })
        .collect();
    dataset.sort_by_key(|(k, _)| *k);

    // Build BPlusTreeMap
    let mut map = BPlusTreeMap::new(cap).expect("new bplustree");
    for &(k, v) in &dataset {
        map.insert(k, v);
    }

    // Get the first key (smallest key in sorted dataset)
    println!("Tree built. Tree has {} items.", map.len());
    println!();

    // Warmup
    println!("Warming up...");
    for _ in 0..100 {
        for (k, v) in map.items().take(iter_count) {
            black_box((k, v));
        }
    }

    println!();
    println!("=== PROFILING RUN ===");
    println!(
        "Scenario: Iterate first {} items from beginning (Scenario 1)",
        iter_count
    );
    println!("Method: Using items() which calls leftmost_leaf() on initialization");
    println!();
    println!("Running 10,000 iterations for profiling...");
    println!();
    println!("To profile with Instruments:");
    println!("  1. Build: cargo build --release --bin profile_from_beginning");
    println!("  2. Run Instruments Time Profiler:");
    println!("     instruments -t 'Time Profiler' -D profile.trace ./target/release/profile_from_beginning");
    println!();

    let start = Instant::now();

    // Run the slow scenario many times for profiling
    let iterations = 10_000;
    let mut total_items = 0;

    for _ in 0..iterations {
        let mut count = 0;
        // This uses items() which calls leftmost_leaf() for initialization
        // This is the exact same code path as bench_partial_iter Scenario 1
        for (k, v) in map.items().take(iter_count) {
            black_box((k, v));
            count += 1;
        }
        total_items += count;
    }

    let elapsed = start.elapsed();

    println!("Done!");
    println!();
    println!("Results:");
    println!("  Iterations: {}", iterations);
    println!("  Items per iteration: {}", iter_count);
    println!("  Total items processed: {}", total_items);
    println!("  Total time: {:.2}s", elapsed.as_secs_f64());
    println!(
        "  Time per iteration: {:.2}ms",
        elapsed.as_secs_f64() * 1000.0 / iterations as f64
    );
    println!(
        "  Time per item: {:.2}µs",
        elapsed.as_secs_f64() * 1_000_000.0 / total_items as f64
    );
}
