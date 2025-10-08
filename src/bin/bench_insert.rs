use std::collections::BTreeMap;
use std::env;
use std::hint::black_box;
use std::time::Duration;
use std::time::Instant;

use bplustree::BPlusTreeMap;
// use old_bplustree::BPlusTreeMap as OldBPlusTreeMap;

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
    // let previous = bench_previous(&dataset, &lookup_keys, cap);
    let std_map = bench_std(&dataset, &lookup_keys);

    println!("\n=== Complete Performance Benchmark ===");
    println!("items: {}  |  bplustree capacity: {}", n, cap);
    println!(
        "{:<18} {:>10} {:>12} {:>10} {:>12} {:>10} {:>12} {:>10} {:>12} {:>10} {:>12}",
        "target",
        "ins(s)",
        "ins Mops",
        "get(s)",
        "get Mops",
        "del(s)",
        "del Mops",
        "mix(s)",
        "mix Mops",
        "iter(s)",
        "iter Mops"
    );
    for result in [current, /* previous, */ std_map] {
        println!(
            "{:<18} {:>10.3} {:>12.2} {:>10.3} {:>12.2} {:>10.3} {:>12.2} {:>10.3} {:>12.2} {:>10.3} {:>12.2}",
            result.label,
            result.insert.as_secs_f64(),
            throughput(n, result.insert),
            result.get.as_secs_f64(),
            throughput(n, result.get),
            result.delete.as_secs_f64(),
            throughput(n, result.delete),
            result.mixed.as_secs_f64(),
            throughput(n, result.mixed),
            result.iterate.as_secs_f64(),
            throughput(n, result.iterate)
        );
    }
}

struct BenchResult {
    label: &'static str,
    insert: Duration,
    get: Duration,
    delete: Duration,
    mixed: Duration,
    iterate: Duration,
}

fn bench_current(dataset: &[(u64, u64)], lookups: &[u64], cap: usize) -> BenchResult {
    let mut map = BPlusTreeMap::new(cap).expect("current new");
    let insert = time_insert(&mut map, dataset);
    let get = time_get(|k| map.get(k), lookups);
    let iterate = time_iterate(&map);

    // For delete benchmark, create a fresh map
    let mut map_for_delete = BPlusTreeMap::new(cap).expect("current new for delete");
    for &(k, v) in dataset {
        map_for_delete.insert(k, v);
    }
    let delete = time_delete(&mut map_for_delete, lookups);

    // For mixed operations benchmark
    let mut map_for_mixed = BPlusTreeMap::new(cap).expect("current new for mixed");
    let mixed = time_mixed_operations(&mut map_for_mixed, dataset, lookups);

    BenchResult {
        label: "bplustree-current",
        insert,
        get,
        delete,
        mixed,
        iterate,
    }
}

// fn bench_previous(dataset: &[(u64, u64)], lookups: &[u64], cap: usize) -> BenchResult {
//     let mut map = OldBPlusTreeMap::new(cap).expect("previous new");
//     let insert = time_insert(&mut map, dataset);
//     let get = time_get(|k| map.get(k), lookups);
//     BenchResult {
//         label: "bplustree-old",
//         insert,
//         get,
//     }
// }

fn bench_std(dataset: &[(u64, u64)], lookups: &[u64]) -> BenchResult {
    let mut map = BTreeMap::new();
    let insert = time_insert(&mut map, dataset);
    let get = time_get(|k| map.get(k), lookups);
    let iterate = time_iterate(&map);

    // For delete benchmark, create a fresh map
    let mut map_for_delete = BTreeMap::new();
    for &(k, v) in dataset {
        map_for_delete.insert(k, v);
    }
    let delete = time_delete(&mut map_for_delete, lookups);

    // For mixed operations benchmark
    let mut map_for_mixed = BTreeMap::new();
    let mixed = time_mixed_operations(&mut map_for_mixed, dataset, lookups);

    BenchResult {
        label: "std::BTreeMap",
        insert,
        get,
        delete,
        mixed,
        iterate,
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

fn time_iterate<M>(map: &M) -> Duration
where
    M: IterateBenchmark,
{
    let start = Instant::now();
    let mut count = 0;
    for (k, v) in map.iter() {
        black_box((k, v));
        count += 1;
    }
    let elapsed = start.elapsed();
    // Ensure we actually iterated over all elements
    black_box(count);
    elapsed
}

fn time_delete<M>(map: &mut M, keys: &[u64]) -> Duration
where
    M: DeleteBenchmark,
{
    let start = Instant::now();
    for k in keys {
        black_box(map.remove(k));
    }
    start.elapsed()
}

fn time_mixed_operations<M>(map: &mut M, dataset: &[(u64, u64)], lookups: &[u64]) -> Duration
where
    M: InsertBenchmark + DeleteBenchmark + GetBenchmark,
{
    let start = Instant::now();

    // Mixed workload: 50% inserts, 30% gets, 20% deletes
    let total_ops = dataset.len();
    let insert_ops = total_ops / 2;
    let get_ops = (total_ops * 3) / 10;
    let delete_ops = total_ops / 5;

    // Insert first half
    for &(k, v) in &dataset[..insert_ops] {
        map.insert(k, v);
    }

    // Mixed operations on the inserted data
    for i in 0..get_ops.min(lookups.len()) {
        black_box(map.get(&lookups[i]));
    }

    // Delete some elements
    for i in 0..delete_ops.min(lookups.len()) {
        black_box(map.remove(&lookups[i]));
    }

    // Insert remaining elements
    for &(k, v) in &dataset[insert_ops..] {
        map.insert(k, v);
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

trait GetBenchmark {
    fn get(&self, key: &u64) -> Option<&u64>;
}

trait DeleteBenchmark {
    fn remove(&mut self, key: &u64) -> Option<u64>;
}

trait IterateBenchmark {
    type Iter<'a>: Iterator<Item = (&'a u64, &'a u64)>
    where
        Self: 'a;
    fn iter(&self) -> Self::Iter<'_>;
}

impl InsertBenchmark for BPlusTreeMap<u64, u64> {
    fn insert(&mut self, key: u64, value: u64) {
        Self::insert(self, key, value);
    }
}

impl GetBenchmark for BPlusTreeMap<u64, u64> {
    fn get(&self, key: &u64) -> Option<&u64> {
        self.get(key)
    }
}

impl DeleteBenchmark for BPlusTreeMap<u64, u64> {
    fn remove(&mut self, key: &u64) -> Option<u64> {
        self.remove(key)
    }
}

impl IterateBenchmark for BPlusTreeMap<u64, u64> {
    type Iter<'a> = bplustree::Items<'a, u64, u64>;
    fn iter(&self) -> Self::Iter<'_> {
        self.items()
    }
}

// impl InsertBenchmark for OldBPlusTreeMap<u64, u64> {
//     fn insert(&mut self, key: u64, value: u64) {
//         Self::insert(self, key, value);
//     }
// }

impl InsertBenchmark for BTreeMap<u64, u64> {
    fn insert(&mut self, key: u64, value: u64) {
        Self::insert(self, key, value);
    }
}

impl GetBenchmark for BTreeMap<u64, u64> {
    fn get(&self, key: &u64) -> Option<&u64> {
        self.get(key)
    }
}

impl DeleteBenchmark for BTreeMap<u64, u64> {
    fn remove(&mut self, key: &u64) -> Option<u64> {
        self.remove(key)
    }
}

impl IterateBenchmark for BTreeMap<u64, u64> {
    type Iter<'a> = std::collections::btree_map::Iter<'a, u64, u64>;
    fn iter(&self) -> Self::Iter<'_> {
        self.iter()
    }
}
