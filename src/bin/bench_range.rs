use bplustree::BPlusTreeMap;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn bench_bplus_range(n: usize, range_size: usize, iterations: usize) -> f64 {
    let mut map = BPlusTreeMap::with_cache_lines(2, 2);
    for i in 0..n {
        map.insert(i, i * 2);
    }

    let start = n / 2 - range_size / 2;
    let end = start + range_size;

    let t0 = Instant::now();
    for _ in 0..iterations {
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }
    t0.elapsed().as_secs_f64()
}

fn bench_std_range(n: usize, range_size: usize, iterations: usize) -> f64 {
    let mut map = BTreeMap::new();
    for i in 0..n {
        map.insert(i, i * 2);
    }

    let start = n / 2 - range_size / 2;
    let end = start + range_size;

    let t0 = Instant::now();
    for _ in 0..iterations {
        let mut sum = 0u64;
        for (k, v) in map.range(start..end) {
            sum = sum.wrapping_add(*k as u64 + *v as u64);
        }
        black_box(sum);
    }
    t0.elapsed().as_secs_f64()
}

fn main() {
    let n = 1_000_000;

    println!("Range Benchmark (n={})", n);
    println!(
        "{:<20} {:<15} {:<15} {:<10}",
        "Range Size", "BPlusTree", "std::BTree", "Speedup"
    );
    println!("{}", "=".repeat(65));

    let configs = vec![
        (100, 10_000),
        (1_000, 1_000),
        (10_000, 100),
        (100_000, 10),
        (n, 5),
    ];

    for (range_size, iterations) in configs {
        let bplus_time = bench_bplus_range(n, range_size, iterations);
        let std_time = bench_std_range(n, range_size, iterations);
        let speedup = std_time / bplus_time;

        println!(
            "{:<20} {:<15.6} {:<15.6} {:<10.2}x",
            format!("{}", range_size),
            bplus_time,
            std_time,
            speedup
        );
    }
}
