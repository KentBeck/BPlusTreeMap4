#[cfg(target_arch = "x86_64")]
use bplustree::BPlusTreeMap;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_rdtsc;
#[cfg(target_arch = "x86_64")]
use std::hint::black_box;

// Measure CPU cycles using RDTSC instruction
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn rdtsc() -> u64 {
    _rdtsc()
}

#[cfg(target_arch = "x86_64")]
fn main() {
    let n = 1_000_000;
    let mut map = BPlusTreeMap::with_cache_lines(2, 2);

    println!("Building tree with {} elements...", n);
    for i in 0..n {
        map.insert(i, i * 2);
    }

    println!("Tree built. Starting cycle-level profiling...\n");

    // Warm up
    for _ in 0..100 {
        let mut sum = 0u64;
        for (k, v) in map.range(500_000..500_100) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }

    println!("=== Small Range (100 elements, 10k iterations) ===");
    let start = n / 2;
    let end = start + 100;
    let iterations = 10_000;

    // Measure iterator creation
    let mut create_cycles = 0u64;
    for _ in 0..iterations {
        let t0 = unsafe { rdtsc() };
        let iter = map.range(start..end);
        let t1 = unsafe { rdtsc() };
        black_box(&iter);
        create_cycles += t1.saturating_sub(t0);
    }

    // Measure first next() call (initialization)
    let mut init_cycles = 0u64;
    for _ in 0..iterations {
        let t0 = unsafe { rdtsc() };
        let mut iter = map.range(start..end);
        let first = iter.next();
        let t1 = unsafe { rdtsc() };
        black_box(first);
        init_cycles += t1.saturating_sub(t0);
    }

    // Measure full iteration
    let mut total_cycles = 0u64;
    for _ in 0..iterations {
        let t0 = unsafe { rdtsc() };
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        let t1 = unsafe { rdtsc() };
        black_box(sum);
        total_cycles += t1.saturating_sub(t0);
    }

    // Measure just the iteration loop (skip first element)
    let _iter_cycles = 0u64;
    for _ in 0..iterations {
        let mut iter = map.range(start..end);
        iter.next(); // Skip first
        let t0 = unsafe { rdtsc() };
        let mut sum = 0u64;
        for (k, v) in iter {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        let t1 = unsafe { rdtsc() };
        black_box(sum);
        _iter_cycles += t1.saturating_sub(t0);
    }

    let avg_create = create_cycles / iterations as u64;
    let avg_init = (init_cycles - create_cycles) / iterations as u64;
    let avg_total = total_cycles / iterations as u64;
    let avg_iter = (total_cycles - init_cycles) / iterations as u64;
    let avg_per_elem = avg_iter / 99; // 99 remaining elements

    println!("Average CPU cycles per iteration:");
    println!("  Iterator creation:     {} cycles", avg_create);
    println!("  First next() call:     {} cycles", avg_init);
    println!("  Remaining 99 elements: {} cycles", avg_iter);
    println!("  Per-element cost:      {} cycles", avg_per_elem);
    println!("  Total:                 {} cycles", avg_total);

    println!("\nBreakdown:");
    let init_pct = (avg_init as f64 / avg_total as f64) * 100.0;
    let iter_pct = (avg_iter as f64 / avg_total as f64) * 100.0;
    println!("  Initialization: {:.1}%", init_pct);
    println!("  Iteration:      {:.1}%", iter_pct);

    // Assuming 3GHz CPU: 1 cycle = 0.33ns
    println!("\nEstimated time (assuming 3GHz CPU):");
    println!("  Per-element: {:.2}ns", avg_per_elem as f64 * 0.33);
    println!("  Total:       {:.2}ns", avg_total as f64 * 0.33);

    // Now profile with detailed breakdown
    println!("\n=== Detailed Per-Element Breakdown ===");

    // Measure individual operations
    let _carve_cycles = 0u64;
    let _bound_check_cycles = 0u64;
    let _pointer_cycles = 0u64;

    // This is approximate - measuring individual operations
    for _ in 0..iterations {
        let mut iter = map.range(start..end);
        iter.next(); // Initialize

        // Try to measure carve_leaf overhead by comparing with/without
        let t0 = unsafe { rdtsc() };
        for _ in 0..10 {
            let _ = iter.next();
        }
        let t1 = unsafe { rdtsc() };
        let _ = t1.saturating_sub(t0);
    }

    println!("Note: Detailed breakdown requires source-level instrumentation");
    println!("See RANGE_LINE_LEVEL_ANALYSIS.md for estimated breakdown");
}

#[cfg(not(target_arch = "x86_64"))]
fn main() {
    eprintln!("This benchmark requires x86_64 architecture for RDTSC instruction.");
    eprintln!("Current architecture: {}", std::env::consts::ARCH);
    std::process::exit(1);
}
