use bplustree::BPlusTreeMap;
use std::hint::black_box;

fn main() {
    // Smaller profiling run for callgrind
    let n = 10_000; // Small enough for callgrind
    let cap = 128;

    // Build the tree
    let mut map = BPlusTreeMap::new(cap).expect("new");
    let mut state: u64 = 0x123456789abcdef0;
    let mut keys = Vec::with_capacity(n);

    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        keys.push(key);
        black_box(map.insert(key, i));
    }

    // Shuffle keys for random access
    let mut lookup_state: u64 = 0xfedcba9876543210;
    for i in 0..n {
        lookup_state = lookup_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let j = (lookup_state as usize) % (n - i);
        keys.swap(i, i + j);
    }

    // Perform lookups (this is what we're profiling)
    for key in keys.iter() {
        black_box(map.get(key));
    }

    black_box(map);
}
