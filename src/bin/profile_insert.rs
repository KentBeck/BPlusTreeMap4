use bplustree::BPlusTreeMap;
use std::hint::black_box;

fn main() {
    // Profile insert-heavy workload with much larger dataset
    let n = 10_000_000; // 10M inserts for better profiling
    let cap = 128;

    println!("Profiling {} inserts with capacity {}", n, cap);

    let mut map = BPlusTreeMap::new(cap).expect("new");

    // Generate random-ish data using LCG
    let mut state: u64 = 0x123456789abcdef0;
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        let value = i;
        black_box(map.insert(key, value));
    }

    println!("Inserted {} items", map.len());

    // Keep the map alive so it doesn't get optimized away
    black_box(map);
}
