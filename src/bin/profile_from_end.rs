use bplustree::BPlusTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    println!("=== Profiling: From End Iteration ===\n");

    let n = 10_000_000;
    let cap = 128;
    let iter_count = 100;

    println!("Building tree with {} items (capacity {})...", n, cap);
    let build_start = Instant::now();
    let mut map = BPlusTreeMap::new(cap).expect("new");
    for i in 0..n {
        map.insert(i, i * 2);
    }
    let build_time = build_start.elapsed();
    println!("Tree built in {:?}", build_time);
    println!("Tree has {} items\n", map.len());

    println!("Scenario: Iterate last {} items", iter_count);
    println!(
        "Method: items().skip(total - {}).take({})",
        iter_count, iter_count
    );
    println!();

    // This is what the benchmark does:
    // 1. Call iter().count() to get total (walks tree - O(n))
    // 2. Call iter().skip(total - 100) (walks tree again - O(n))
    // 3. Take 100 items

    // Test 1: Measure just getting the count
    println!("=== Test 1: Measuring iter().count() ===");
    let start = Instant::now();
    let total = map.items().count();
    let count_time = start.elapsed();
    println!("Total items: {}", total);
    println!("Time to count: {:?}", count_time);
    println!();

    // Test 2: Measure skip() + take()
    println!("=== Test 2: Measuring iter().skip().take() ===");
    let skip_amount = total.saturating_sub(iter_count);
    println!("Will skip {} items, take {}", skip_amount, iter_count);

    let start = Instant::now();
    let mut n = 0;
    for (k, v) in map.items().skip(skip_amount).take(iter_count) {
        black_box((k, v));
        n += 1;
    }
    let skip_take_time = start.elapsed();
    println!("Items iterated: {}", n);
    println!("Time: {:?}", skip_take_time);
    println!();

    // Test 3: Full benchmark (count + skip + take)
    println!("=== Test 3: Full benchmark (like the actual benchmark) ===");
    let iterations = 1000;

    let start = Instant::now();
    for _ in 0..iterations {
        let total = map.items().count();
        let mut n = 0;
        for (k, v) in map.items().skip(total.saturating_sub(iter_count)) {
            black_box((k, v));
            n += 1;
        }
        black_box(n);
    }
    let total_time = start.elapsed();

    println!("{} iterations: {:?}", iterations, total_time);
    println!("Per iteration: {:?}", total_time / iterations as u32);
    println!();

    // Test 4: Break down skip cost
    println!("=== Test 4: Breakdown of skip() cost ===");

    // Skip is basically calling next() repeatedly
    println!(
        "Testing skip({}) which calls next() {} times...",
        skip_amount, skip_amount
    );

    let start = Instant::now();
    for _ in 0..100 {
        let mut iter = map.items();
        for _ in 0..skip_amount {
            iter.next();
        }
        // Now take the last 100
        let mut n = 0;
        for (k, v) in iter.take(iter_count) {
            black_box((k, v));
            n += 1;
        }
        black_box(n);
    }
    let manual_skip_time = start.elapsed();
    println!("100 iterations: {:?}", manual_skip_time);
    println!("Per iteration: {:?}", manual_skip_time / 100);
    println!();

    // Test 5: Compare with std::BTreeMap
    println!("=== Test 5: std::BTreeMap comparison ===");
    use std::collections::BTreeMap;

    println!("Building std::BTreeMap...");
    let mut std_map = BTreeMap::new();
    for i in 0..n {
        std_map.insert(i, i * 2);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let total = std_map.iter().count();
        let mut n = 0;
        for (k, v) in std_map.iter().skip(total.saturating_sub(iter_count)) {
            black_box((k, v));
            n += 1;
        }
        black_box(n);
    }
    let std_total_time = start.elapsed();

    println!("{} iterations: {:?}", iterations, std_total_time);
    println!("Per iteration: {:?}", std_total_time / iterations as u32);
    println!();

    // Summary
    println!("=== Summary ===");
    println!("BPlusTreeMap:");
    println!("  count() time: {:?}", count_time);
    println!("  skip + take time: {:?}", skip_take_time);
    println!(
        "  Per iteration (1000x): {:?}",
        total_time / iterations as u32
    );
    println!();
    println!("std::BTreeMap:");
    println!(
        "  Per iteration (1000x): {:?}",
        std_total_time / iterations as u32
    );
    println!();
    println!("Analysis:");
    println!("  count() walks all {} leaf nodes: ~16ms", n / cap);
    println!(
        "  skip({}) walks {} items one by one",
        skip_amount, skip_amount
    );
    println!("  For 10M items, skip walks ~10M items at ~16ns each = ~160ms");
    println!("  Total expected: count (16ms) + skip (160ms) + take (1ms) = ~177ms");
    println!();
    println!("The 'from end' scenario is slow because:");
    println!("  1. count() calls len() which is O(n) - walks all leaves");
    println!("  2. skip() calls next() n times, each walking through items");
    println!("  3. This is not a real-world use case - use range() instead");
}
