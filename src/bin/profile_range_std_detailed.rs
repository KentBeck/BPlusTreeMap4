use std::collections::BTreeMap;
use std::hint::black_box;

fn main() {
    let n = 1_000_000;
    let mut map = BTreeMap::new();

    println!("Building tree with {} elements...", n);
    for i in 0..n {
        map.insert(i, i * 2);
    }

    println!("Tree built. Starting range iteration profiling...\n");

    // Profile small ranges (100 elements) - many iterations to get good samples
    println!("=== Small Range (100 elements, 100k iterations) ===");
    let start = n / 2;
    let end = start + 100;

    for _ in 0..100_000 {
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }

    println!("Small range profiling complete\n");

    // Profile medium ranges (10k elements) - moderate iterations
    println!("=== Medium Range (10k elements, 1k iterations) ===");
    let start = n / 4;
    let end = start + 10_000;

    for _ in 0..1_000 {
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }

    println!("Medium range profiling complete\n");

    // Profile large ranges (100k elements) - fewer iterations
    println!("=== Large Range (100k elements, 100 iterations) ===");
    let start = n / 4;
    let end = start + 100_000;

    for _ in 0..100 {
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }

    println!("Large range profiling complete\n");

    println!("All profiling complete. Analyze with perf or samply.");
}
