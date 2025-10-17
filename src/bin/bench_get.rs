use bplustree::BPlusTreeMap;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn bench_bplustree_get(n: usize, cap: usize) -> (f64, f64) {
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

    // Shuffle keys for random access pattern
    let mut lookup_state: u64 = 0xfedcba9876543210;
    for i in 0..n {
        lookup_state = lookup_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let j = (lookup_state as usize) % (n - i);
        keys.swap(i, i + j);
    }

    // Perform lookups
    let get_start = Instant::now();
    for key in keys.iter() {
        black_box(map.get(key));
    }
    let get_time = get_start.elapsed().as_secs_f64();

    black_box(map);
    (build_time, get_time)
}

fn bench_std_btree_get(n: usize) -> (f64, f64) {
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

    // Shuffle keys for random access pattern
    let mut lookup_state: u64 = 0xfedcba9876543210;
    for i in 0..n {
        lookup_state = lookup_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let j = (lookup_state as usize) % (n - i);
        keys.swap(i, i + j);
    }

    // Perform lookups
    let get_start = Instant::now();
    for key in keys.iter() {
        black_box(map.get(key));
    }
    let get_time = get_start.elapsed().as_secs_f64();

    black_box(map);
    (build_time, get_time)
}

fn main() {
    let sizes = vec![10_000, 100_000, 1_000_000];
    let cap = 128;

    println!("Get Performance Benchmark");
    println!("=========================\n");

    for &n in &sizes {
        println!("Testing with {} items:", n);

        // Warmup
        let _ = bench_bplustree_get(1000, cap);
        let _ = bench_std_btree_get(1000);

        // BPlusTreeMap
        let (build_bp, get_bp) = bench_bplustree_get(n, cap);
        println!("  BPlusTreeMap (cap={}):", cap);
        println!(
            "    Build: {:.3}s ({:.0} ops/sec)",
            build_bp,
            n as f64 / build_bp
        );
        println!(
            "    Get:   {:.3}s ({:.0} ops/sec)",
            get_bp,
            n as f64 / get_bp
        );

        // std::BTreeMap
        let (build_std, get_std) = bench_std_btree_get(n);
        println!("  std::BTreeMap:");
        println!(
            "    Build: {:.3}s ({:.0} ops/sec)",
            build_std,
            n as f64 / build_std
        );
        println!(
            "    Get:   {:.3}s ({:.0} ops/sec)",
            get_std,
            n as f64 / get_std
        );

        // Comparison
        let get_ratio = get_bp / get_std;
        println!("  Ratio (BPlusTree/std):");
        println!(
            "    Get: {:.2}x {}",
            get_ratio,
            if get_ratio < 1.0 {
                "(faster)"
            } else {
                "(slower)"
            }
        );
        println!();
    }
}
