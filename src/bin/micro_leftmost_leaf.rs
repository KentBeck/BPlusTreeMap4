use bplustree::BPlusTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    println!("=== Micro-benchmark: leftmost_leaf() isolation ===\n");

    let n = 10_000_000;
    let cap = 128;

    println!("Building tree with {} items (capacity {})...", n, cap);
    let build_start = Instant::now();
    let mut map = BPlusTreeMap::new(cap).expect("new");
    for i in 0..n {
        map.insert(i, i * 2);
    }
    let build_time = build_start.elapsed();
    println!("Tree built in {:?}", build_time);
    println!("Tree has {} items\n", map.len());

    // Calculate tree height
    let leaf_nodes = (n + cap - 1) / cap;
    let mut height = 1;
    let mut nodes_at_level = leaf_nodes;
    while nodes_at_level > 1 {
        nodes_at_level = (nodes_at_level + cap - 1) / cap;
        height += 1;
    }
    println!("Calculated tree height: {} levels", height);
    println!("Expected cost: {}ns ({}ns per level)\n", height * 20, 20);

    // Test 1: Measure items() iterator creation only (no consumption)
    println!("=== Test 1: items() creation (no iteration) ===");
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let iter = map.items();
        black_box(&iter);
        drop(iter);
    }
    let elapsed = start.elapsed();
    println!("{} items() calls: {:?}", iterations, elapsed);
    println!("Per call: {}ns", elapsed.as_nanos() / iterations);
    println!();

    // Test 2: Measure full iteration of first 100 items
    println!("=== Test 2: Full iteration (100 items each) ===");
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut count = 0;
        for (k, v) in map.items() {
            black_box((k, v));
            count += 1;
            if count >= 100 {
                break;
            }
        }
    }
    let elapsed = start.elapsed();
    println!("{} iterations: {:?}", iterations, elapsed);
    println!("Per iteration: {}μs", elapsed.as_micros() / iterations);
    println!("Per item: {}ns", elapsed.as_nanos() / (iterations * 100));
    println!();

    // Test 3: Measure iteration with take() (like the benchmark)
    println!("=== Test 3: Iteration with take(100) ===");
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut n = 0;
        for (k, v) in map.items().take(100) {
            black_box((k, v));
            n += 1;
        }
        black_box(n);
    }
    let elapsed = start.elapsed();
    println!("{} iterations: {:?}", iterations, elapsed);
    println!("Per iteration: {}μs", elapsed.as_micros() / iterations);
    println!();

    // Test 4: Compare with range() from first key
    println!("=== Test 4: Compare range() vs items() ===");
    let first_key = 0; // We know the first key is 0

    println!("Testing range(0..).take(100):");
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut n = 0;
        for (k, v) in map.range(first_key..).take(100) {
            black_box((k, v));
            n += 1;
        }
        black_box(n);
    }
    let elapsed_range = start.elapsed();
    println!("  {} iterations: {:?}", iterations, elapsed_range);
    println!(
        "  Per iteration: {}μs",
        elapsed_range.as_micros() / iterations
    );

    println!("\nTesting items().take(100):");
    let start = Instant::now();
    for _ in 0..iterations {
        let mut n = 0;
        for (k, v) in map.items().take(100) {
            black_box((k, v));
            n += 1;
        }
        black_box(n);
    }
    let elapsed_items = start.elapsed();
    println!("  {} iterations: {:?}", iterations, elapsed_items);
    println!(
        "  Per iteration: {}μs",
        elapsed_items.as_micros() / iterations
    );

    let diff = elapsed_items
        .as_nanos()
        .saturating_sub(elapsed_range.as_nanos());
    println!("\nDifference: {}ns per iteration", diff / iterations);
    println!();

    // Test 5: Single iteration timing (like the benchmark does)
    println!("=== Test 5: Single iteration (replicating benchmark) ===");
    for trial in 1..=10 {
        let start = Instant::now();
        let mut iter = map.items();
        let mut n = 0;
        for (k, v) in iter.by_ref().take(100) {
            black_box((k, v));
            n += 1;
        }
        let elapsed = start.elapsed();
        black_box(n);
        println!("Trial {}: {:?} ({} items)", trial, elapsed, n);
    }
    println!();

    // Test 6: std::BTreeMap comparison
    println!("=== Test 6: std::BTreeMap comparison ===");
    use std::collections::BTreeMap;

    println!("Building std::BTreeMap with {} items...", n);
    let build_start = Instant::now();
    let mut std_map = BTreeMap::new();
    for i in 0..n {
        std_map.insert(i, i * 2);
    }
    println!("Built in {:?}\n", build_start.elapsed());

    println!("Testing std::BTreeMap iter().take(100):");
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut n = 0;
        for (k, v) in std_map.iter().take(100) {
            black_box((k, v));
            n += 1;
        }
        black_box(n);
    }
    let elapsed_std = start.elapsed();
    println!("  {} iterations: {:?}", iterations, elapsed_std);
    println!(
        "  Per iteration: {}μs",
        elapsed_std.as_micros() / iterations
    );
    println!(
        "  Per item: {}ns",
        elapsed_std.as_nanos() / (iterations * 100)
    );

    println!("\n=== Summary ===");
    println!(
        "BPlusTreeMap items().take(100): {}μs per iteration",
        elapsed_items.as_micros() / iterations
    );
    println!(
        "BPlusTreeMap range(0..).take(100): {}μs per iteration",
        elapsed_range.as_micros() / iterations
    );
    println!(
        "std::BTreeMap iter().take(100): {}μs per iteration",
        elapsed_std.as_micros() / iterations
    );

    let slowdown_vs_std =
        (elapsed_items.as_nanos() / iterations) / (elapsed_std.as_nanos() / iterations);
    println!(
        "\nBPlusTreeMap is {}x slower than std::BTreeMap",
        slowdown_vs_std
    );

    if elapsed_items > elapsed_range {
        let overhead = (elapsed_items.as_nanos() - elapsed_range.as_nanos()) / iterations;
        println!("items() has {}ns overhead vs range()", overhead);
    }
}
