use std::collections::BTreeMap;
use std::env;
use std::hint::black_box;
use std::time::Duration;
use std::time::Instant;

use bplustree::BPlusTreeMap;
use old_bplustree::BPlusTreeMap as OldBPlusTreeMap;

fn parse_arg<T: std::str::FromStr>(i: usize, default: T) -> T {
    env::args()
        .nth(i)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    // Usage: bench_insert [n=1000000] [cap=16]
    let n: usize = parse_arg(1, 1_000_000);
    let cap: usize = parse_arg(2, 16);

    let dataset = generate_dataset(n);
    let lookup_keys: Vec<u64> = dataset.iter().map(|(k, _)| *k).collect();

    let current = bench_current(&dataset, &lookup_keys, cap);
    let previous = bench_previous(&dataset, &lookup_keys, cap);
    let std_map = bench_std(&dataset, &lookup_keys);

    println!("\n=== Insert/Get Benchmark ===");
    println!("items: {}  |  bplustree capacity: {}", n, cap);
    println!(
        "{:<18} {:>12} {:>15} {:>12} {:>15}",
        "target", "insert(s)", "insert Mops", "get(s)", "get Mops"
    );
    for result in [current, previous, std_map] {
        println!(
            "{:<18} {:>12.3} {:>15.2} {:>12.3} {:>15.2}",
            result.label,
            result.insert.as_secs_f64(),
            throughput(n, result.insert),
            result.get.as_secs_f64(),
            throughput(n, result.get)
        );
    }
}

struct BenchResult {
    label: &'static str,
    insert: Duration,
    get: Duration,
}

fn bench_current(dataset: &[(u64, u64)], lookups: &[u64], cap: usize) -> BenchResult {
    let mut map = BPlusTreeMap::new(cap).expect("current new");
    let insert = time_insert(&mut map, dataset);
    let get = time_get(|k| map.get(k), lookups);
    BenchResult {
        label: "bplustree-current",
        insert,
        get,
    }
}

fn bench_previous(dataset: &[(u64, u64)], lookups: &[u64], cap: usize) -> BenchResult {
    let mut map = OldBPlusTreeMap::new(cap).expect("previous new");
    let insert = time_insert(&mut map, dataset);
    let get = time_get(|k| map.get(k), lookups);
    BenchResult {
        label: "bplustree-old",
        insert,
        get,
    }
}

fn bench_std(dataset: &[(u64, u64)], lookups: &[u64]) -> BenchResult {
    let mut map = BTreeMap::new();
    let insert = time_insert(&mut map, dataset);
    let get = time_get(|k| map.get(k), lookups);
    BenchResult {
        label: "std::BTreeMap",
        insert,
        get,
    }
}

fn generate_dataset(n: usize) -> Vec<(u64, u64)> {
    let mut state: u64 = 0x9E3779B97F4A7C15;
    (0..n as u64)
        .map(|i| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state, i)
        })
        .collect()
}

fn time_insert<M>(map: &mut M, dataset: &[(u64, u64)]) -> Duration
where
    M: InsertBenchmark,
{
    let start = Instant::now();
    for &(k, v) in dataset {
        map.insert(k, v);
    }
    start.elapsed()
}

fn time_get<F, R>(mut get_fn: F, lookups: &[u64]) -> Duration
where
    F: FnMut(&u64) -> R,
{
    let start = Instant::now();
    for k in lookups {
        black_box(get_fn(k));
    }
    start.elapsed()
}

fn throughput(count: usize, duration: Duration) -> f64 {
    let secs = duration.as_secs_f64().max(1e-9);
    (count as f64 / 1_000_000.0) / secs
}

trait InsertBenchmark {
    fn insert(&mut self, key: u64, value: u64);
}

impl InsertBenchmark for BPlusTreeMap<u64, u64> {
    fn insert(&mut self, key: u64, value: u64) {
        Self::insert(self, key, value);
    }
}

impl InsertBenchmark for OldBPlusTreeMap<u64, u64> {
    fn insert(&mut self, key: u64, value: u64) {
        Self::insert(self, key, value);
    }
}

impl InsertBenchmark for BTreeMap<u64, u64> {
    fn insert(&mut self, key: u64, value: u64) {
        Self::insert(self, key, value);
    }
}
