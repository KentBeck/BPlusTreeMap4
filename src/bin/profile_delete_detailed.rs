use bplustree::BPlusTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    // Detailed profiling of delete operations with timing breakdown
    let n = 1_000_000;
    let cap = 128;

    println!("Detailed Delete Profiling ({} operations)", n);
    println!("==========================================\n");

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

    println!("Built tree with {} items", map.len());
    println!("Leaf count: {}", map.leaf_count());
    println!();

    // Shuffle keys
    let mut delete_state: u64 = 0xfedcba9876543210;
    for i in 0..n {
        delete_state = delete_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let j = (delete_state as usize) % (n - i);
        keys.swap(i, i + j);
    }

    // Profile delete operations in batches
    let batch_size = 100_000;
    let mut total_time = 0.0;

    println!("Deleting in batches of {}:", batch_size);
    for batch in 0..(n / batch_size) {
        let start_idx = batch * batch_size;
        let end_idx = (batch + 1) * batch_size;

        let start = Instant::now();
        for i in start_idx..end_idx {
            black_box(map.remove(&keys[i]));
        }
        let elapsed = start.elapsed().as_secs_f64();
        total_time += elapsed;

        let remaining = map.len();
        let ops_per_sec = batch_size as f64 / elapsed;
        println!(
            "  Batch {}: {:.3}s ({:.0} ops/sec, {} items remaining)",
            batch + 1,
            elapsed,
            ops_per_sec,
            remaining
        );
    }

    println!();
    println!("Total delete time: {:.3}s", total_time);
    println!("Average: {:.0} ops/sec", n as f64 / total_time);
    println!("Final len: {}", map.len());

    black_box(map);
}
