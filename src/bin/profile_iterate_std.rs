use std::collections::BTreeMap;
use std::hint::black_box;

fn main() {
    // Profile iteration workload with std::BTreeMap
    let n = 1_000_000;

    println!("Profiling {} iteration operations with std::BTreeMap", n);

    // Phase 1: Build the tree
    println!("Phase 1: Building tree with {} items...", n);
    let mut map = BTreeMap::new();

    let mut state: u64 = 0x123456789abcdef0;
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        black_box(map.insert(key, i));
    }

    println!("Built tree with {} items", map.len());

    // Phase 2: Forward iteration (this is what we're profiling)
    println!("Phase 2: Forward iteration...");
    let mut count = 0;
    for (k, v) in map.iter() {
        black_box(k);
        black_box(v);
        count += 1;
    }
    println!("Forward iteration complete: {} items", count);

    // Phase 3: Backward iteration
    println!("Phase 3: Backward iteration...");
    let mut count = 0;
    for (k, v) in map.iter().rev() {
        black_box(k);
        black_box(v);
        count += 1;
    }
    println!("Backward iteration complete: {} items", count);

    // Keep the map alive
    black_box(map);
}
