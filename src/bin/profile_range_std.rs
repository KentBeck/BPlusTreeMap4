use std::collections::BTreeMap;
use std::hint::black_box;

fn main() {
    let n = 1_000_000;
    let mut map = BTreeMap::new();

    // Insert sequential keys
    for i in 0..n {
        map.insert(i, i * 2);
    }

    // Profile various range queries
    println!("Profiling std::BTreeMap range queries on {} elements", n);

    // Small range (100 elements)
    for _ in 0..10_000 {
        let start = n / 2;
        let end = start + 100;
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }

    // Medium range (10,000 elements)
    for _ in 0..100 {
        let start = n / 4;
        let end = start + 10_000;
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }

    // Large range (100,000 elements)
    for _ in 0..10 {
        let start = n / 4;
        let end = start + 100_000;
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }

    // Full range
    for _ in 0..5 {
        let mut sum = 0u64;
        for (k, v) in map.range(..) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }

    println!("std::BTreeMap range profiling complete");
}
