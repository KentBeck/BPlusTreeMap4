use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let n = 1_000_000;
    let mut map = BTreeMap::new();

    println!("Building tree with {} elements...", n);
    for i in 0..n {
        map.insert(i, i * 2);
    }

    println!("Tree built. Starting detailed timing analysis...\n");

    // Small range analysis
    println!("=== Small Range (100 elements, 100k iterations) ===");
    let start = n / 2;
    let end = start + 100;
    let iterations = 100_000;

    // Time just iterator creation
    let t0 = Instant::now();
    for _ in 0..iterations {
        let iter = map.range(start..end);
        black_box(&iter);
    }
    let create_time = t0.elapsed().as_secs_f64();

    // Time iterator creation + first next()
    let t0 = Instant::now();
    for _ in 0..iterations {
        let mut iter = map.range(start..end);
        let first = iter.next();
        black_box(first);
    }
    let first_next_time = t0.elapsed().as_secs_f64();

    // Time full iteration
    let t0 = Instant::now();
    for _ in 0..iterations {
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }
    let full_time = t0.elapsed().as_secs_f64();

    println!("Results per iteration:");
    println!(
        "  Iterator creation:     {:.2}µs",
        create_time * 1_000_000.0 / iterations as f64
    );
    println!(
        "  First next() call:     {:.2}µs",
        (first_next_time - create_time) * 1_000_000.0 / iterations as f64
    );
    println!(
        "  Remaining 99 elements: {:.2}µs",
        (full_time - first_next_time) * 1_000_000.0 / iterations as f64
    );
    println!(
        "  Per-element cost:      {:.2}ns",
        (full_time - first_next_time) * 1_000_000_000.0 / (iterations as f64 * 99.0)
    );
    println!(
        "  Total time:            {:.2}µs",
        full_time * 1_000_000.0 / iterations as f64
    );

    println!("\nBreakdown:");
    let init_pct = (first_next_time - create_time) / full_time * 100.0;
    let iter_pct = (full_time - first_next_time) / full_time * 100.0;
    println!("  Initialization: {:.1}%", init_pct);
    println!("  Iteration:      {:.1}%", iter_pct);

    // Medium range analysis
    println!("\n=== Medium Range (10k elements, 10k iterations) ===");
    let start = n / 4;
    let end = start + 10_000;
    let iterations = 10_000;

    let t0 = Instant::now();
    for _ in 0..iterations {
        let iter = map.range(start..end);
        black_box(&iter);
    }
    let create_time = t0.elapsed().as_secs_f64();

    let t0 = Instant::now();
    for _ in 0..iterations {
        let mut iter = map.range(start..end);
        let first = iter.next();
        black_box(first);
    }
    let first_next_time = t0.elapsed().as_secs_f64();

    let t0 = Instant::now();
    for _ in 0..iterations {
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }
    let full_time = t0.elapsed().as_secs_f64();

    println!("Results per iteration:");
    println!(
        "  Iterator creation:     {:.2}µs",
        create_time * 1_000_000.0 / iterations as f64
    );
    println!(
        "  First next() call:     {:.2}µs",
        (first_next_time - create_time) * 1_000_000.0 / iterations as f64
    );
    println!(
        "  Remaining 9999 elements: {:.2}µs",
        (full_time - first_next_time) * 1_000_000.0 / iterations as f64
    );
    println!(
        "  Per-element cost:      {:.2}ns",
        (full_time - first_next_time) * 1_000_000_000.0 / (iterations as f64 * 9999.0)
    );
    println!(
        "  Total time:            {:.2}µs",
        full_time * 1_000_000.0 / iterations as f64
    );

    println!("\nBreakdown:");
    let init_pct = (first_next_time - create_time) / full_time * 100.0;
    let iter_pct = (full_time - first_next_time) / full_time * 100.0;
    println!("  Initialization: {:.1}%", init_pct);
    println!("  Iteration:      {:.1}%", iter_pct);

    // Large range analysis
    println!("\n=== Large Range (100k elements, 1k iterations) ===");
    let start = n / 4;
    let end = start + 100_000;
    let iterations = 1_000;

    let t0 = Instant::now();
    for _ in 0..iterations {
        let iter = map.range(start..end);
        black_box(&iter);
    }
    let create_time = t0.elapsed().as_secs_f64();

    let t0 = Instant::now();
    for _ in 0..iterations {
        let mut iter = map.range(start..end);
        let first = iter.next();
        black_box(first);
    }
    let first_next_time = t0.elapsed().as_secs_f64();

    let t0 = Instant::now();
    for _ in 0..iterations {
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }
    let full_time = t0.elapsed().as_secs_f64();

    println!("Results per iteration:");
    println!(
        "  Iterator creation:     {:.2}µs",
        create_time * 1_000_000.0 / iterations as f64
    );
    println!(
        "  First next() call:     {:.2}µs",
        (first_next_time - create_time) * 1_000_000.0 / iterations as f64
    );
    println!(
        "  Remaining 99999 elements: {:.2}µs",
        (full_time - first_next_time) * 1_000_000.0 / iterations as f64
    );
    println!(
        "  Per-element cost:      {:.2}ns",
        (full_time - first_next_time) * 1_000_000_000.0 / (iterations as f64 * 99999.0)
    );
    println!(
        "  Total time:            {:.2}µs",
        full_time * 1_000_000.0 / iterations as f64
    );

    println!("\nBreakdown:");
    let init_pct = (first_next_time - create_time) / full_time * 100.0;
    let iter_pct = (full_time - first_next_time) / full_time * 100.0;
    println!("  Initialization: {:.1}%", init_pct);
    println!("  Iteration:      {:.1}%", iter_pct);
}
