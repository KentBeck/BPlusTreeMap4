use std::collections::BTreeMap;
use std::hint::black_box;

fn main() {
    // Profile insert-heavy workload with std::BTreeMap
    let n = 10_000_000; // Match profile_insert.rs

    println!("Profiling {} inserts with std::BTreeMap", n);

    let mut map = BTreeMap::new();

    // Generate random-ish data using LCG (same as profile_insert)
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
