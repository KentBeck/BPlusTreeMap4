use bplustree::BPlusTreeMap;
use std::hint::black_box;

fn main() {
    // Profile delete-heavy workload with detailed line-level profiling
    let n = 1_000_000; // 1M operations for profiling
    let cap = 128;

    println!("Profiling {} delete operations with capacity {}", n, cap);

    // Phase 1: Build the tree
    println!("Phase 1: Building tree with {} items...", n);
    let mut map = BPlusTreeMap::new(cap).expect("new");

    // Generate random-ish data using LCG
    let mut state: u64 = 0x123456789abcdef0;
    let mut keys = Vec::with_capacity(n);
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        keys.push(key);
        black_box(map.insert(key, i));
    }

    println!("Built tree with {} items", map.len());

    // Phase 2: Delete all items (this is what we're profiling)
    println!("Phase 2: Deleting {} items...", n);

    // Shuffle keys for more realistic delete pattern
    let mut delete_state: u64 = 0xfedcba9876543210;
    for i in 0..n {
        delete_state = delete_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let j = (delete_state as usize) % (n - i);
        keys.swap(i, i + j);
    }

    // Now delete in shuffled order
    for key in keys.iter() {
        black_box(map.remove(key));
    }

    println!("Deleted all items, final len: {}", map.len());

    // Keep the map alive
    black_box(map);
}
