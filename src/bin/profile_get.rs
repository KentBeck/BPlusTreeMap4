use bplustree::BPlusTreeMap;
use std::hint::black_box;

fn main() {
    // Profile get-heavy workload
    let n = 1_000_000; // 1M operations for profiling
    let cap = 128;

    println!("Profiling {} get operations with capacity {}", n, cap);

    // Phase 1: Build the tree
    println!("Phase 1: Building tree with {} items...", n);
    let mut map = BPlusTreeMap::new(cap).expect("new");

    let mut state: u64 = 0x123456789abcdef0;
    let mut keys = Vec::with_capacity(n);
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        keys.push(key);
        black_box(map.insert(key, i));
    }

    println!("Built tree with {} items", map.len());

    // Phase 2: Shuffle keys for random access
    println!("Phase 2: Shuffling keys...");
    let mut lookup_state: u64 = 0xfedcba9876543210;
    for i in 0..n {
        lookup_state = lookup_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let j = (lookup_state as usize) % (n - i);
        keys.swap(i, i + j);
    }

    // Phase 3: Perform lookups (this is what we're profiling)
    println!("Phase 3: Performing {} lookups...", n);
    for key in keys.iter() {
        black_box(map.get(key));
    }

    println!("Completed all lookups");

    // Keep the map alive
    black_box(map);
}
