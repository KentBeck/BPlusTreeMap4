use bplustree::BPlusTreeMap;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn bench_bplustree_delete(n: usize, cap: usize) -> (f64, f64) {
    // Build the tree
    let mut map = BPlusTreeMap::new(cap).expect("new");
    let mut state: u64 = 0x123456789abcdef0;
    let mut keys = Vec::with_capacity(n);

    let build_start = Instant::now();
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        keys.push(key);
        black_box(map.insert(key, i));
    }
    let build_time = build_start.elapsed().as_secs_f64();

    // Shuffle keys
    let mut delete_state: u64 = 0xfedcba9876543210;
    for i in 0..n {
        delete_state = delete_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let j = (delete_state as usize) % (n - i);
        keys.swap(i, i + j);
    }

    // Delete all items
    let delete_start = Instant::now();
    for key in keys.iter() {
        black_box(map.remove(key));
    }
    let delete_time = delete_start.elapsed().as_secs_f64();

    black_box(map);
    (build_time, delete_time)
}

fn bench_std_btree_delete(n: usize) -> (f64, f64) {
    // Build the tree
    let mut map = BTreeMap::new();
    let mut state: u64 = 0x123456789abcdef0;
    let mut keys = Vec::with_capacity(n);

    let build_start = Instant::now();
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        keys.push(key);
        black_box(map.insert(key, i));
    }
    let build_time = build_start.elapsed().as_secs_f64();

    // Shuffle keys
    let mut delete_state: u64 = 0xfedcba9876543210;
    for i in 0..n {
        delete_state = delete_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let j = (delete_state as usize) % (n - i);
        keys.swap(i, i + j);
    }

    // Delete all items
    let delete_start = Instant::now();
    for key in keys.iter() {
        black_box(map.remove(key));
    }
    let delete_time = delete_start.elapsed().as_secs_f64();

    black_box(map);
    (build_time, delete_time)
}

fn main() {
    let sizes = vec![10_000, 100_000, 1_000_000];
    let cap = 128;

    println!("Delete Performance Benchmark");
    println!("============================\n");

    for &n in &sizes {
        println!("Testing with {} items:", n);

        // Warmup
        let _ = bench_bplustree_delete(1000, cap);
        let _ = bench_std_btree_delete(1000);

        // BPlusTreeMap
        let (build_bp, delete_bp) = bench_bplustree_delete(n, cap);
        println!("  BPlusTreeMap (cap={}):", cap);
        println!(
            "    Build:  {:.3}s ({:.0} ops/sec)",
            build_bp,
            n as f64 / build_bp
        );
        println!(
            "    Delete: {:.3}s ({:.0} ops/sec)",
            delete_bp,
            n as f64 / delete_bp
        );

        // std::BTreeMap
        let (build_std, delete_std) = bench_std_btree_delete(n);
        println!("  std::BTreeMap:");
        println!(
            "    Build:  {:.3}s ({:.0} ops/sec)",
            build_std,
            n as f64 / build_std
        );
        println!(
            "    Delete: {:.3}s ({:.0} ops/sec)",
            delete_std,
            n as f64 / delete_std
        );

        // Comparison
        let delete_ratio = delete_bp / delete_std;
        println!("  Ratio (BPlusTree/std):");
        println!(
            "    Delete: {:.2}x {}",
            delete_ratio,
            if delete_ratio < 1.0 {
                "(faster)"
            } else {
                "(slower)"
            }
        );
        println!();
    }
}
