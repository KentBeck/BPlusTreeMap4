use bplustree::BPlusTreeMap;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;

fn bench_bplustree_iterate(n: usize, cap: usize) -> (f64, f64, f64) {
    // Build the tree
    let mut map = BPlusTreeMap::new(cap).expect("new");
    let mut state: u64 = 0x123456789abcdef0;

    let build_start = Instant::now();
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        black_box(map.insert(key, i));
    }
    let build_time = build_start.elapsed().as_secs_f64();

    // Forward iteration
    let forward_start = Instant::now();
    let mut count = 0;
    for (k, v) in map.items() {
        black_box(k);
        black_box(v);
        count += 1;
    }
    let forward_time = forward_start.elapsed().as_secs_f64();
    assert_eq!(count, n);

    // Backward iteration
    let backward_start = Instant::now();
    let mut count = 0;
    for (k, v) in map.items().rev() {
        black_box(k);
        black_box(v);
        count += 1;
    }
    let backward_time = backward_start.elapsed().as_secs_f64();
    assert_eq!(count, n);

    black_box(map);
    (build_time, forward_time, backward_time)
}

fn bench_std_btree_iterate(n: usize) -> (f64, f64, f64) {
    // Build the tree
    let mut map = BTreeMap::new();
    let mut state: u64 = 0x123456789abcdef0;

    let build_start = Instant::now();
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = state;
        black_box(map.insert(key, i));
    }
    let build_time = build_start.elapsed().as_secs_f64();

    // Forward iteration
    let forward_start = Instant::now();
    let mut count = 0;
    for (k, v) in map.iter() {
        black_box(k);
        black_box(v);
        count += 1;
    }
    let forward_time = forward_start.elapsed().as_secs_f64();
    assert_eq!(count, n);

    // Backward iteration
    let backward_start = Instant::now();
    let mut count = 0;
    for (k, v) in map.iter().rev() {
        black_box(k);
        black_box(v);
        count += 1;
    }
    let backward_time = backward_start.elapsed().as_secs_f64();
    assert_eq!(count, n);

    black_box(map);
    (build_time, forward_time, backward_time)
}

fn main() {
    let sizes = vec![10_000, 100_000, 1_000_000];
    let cap = 128;

    println!("Iteration Performance Benchmark");
    println!("================================\n");

    for &n in &sizes {
        println!("Testing with {} items:", n);

        // Warmup
        let _ = bench_bplustree_iterate(1000, cap);
        let _ = bench_std_btree_iterate(1000);

        // BPlusTreeMap
        let (build_bp, forward_bp, backward_bp) = bench_bplustree_iterate(n, cap);
        println!("  BPlusTreeMap (cap={}):", cap);
        println!(
            "    Build:    {:.3}s ({:.0} ops/sec)",
            build_bp,
            n as f64 / build_bp
        );
        println!(
            "    Forward:  {:.3}s ({:.0} ops/sec)",
            forward_bp,
            n as f64 / forward_bp
        );
        println!(
            "    Backward: {:.3}s ({:.0} ops/sec)",
            backward_bp,
            n as f64 / backward_bp
        );

        // std::BTreeMap
        let (build_std, forward_std, backward_std) = bench_std_btree_iterate(n);
        println!("  std::BTreeMap:");
        println!(
            "    Build:    {:.3}s ({:.0} ops/sec)",
            build_std,
            n as f64 / build_std
        );
        println!(
            "    Forward:  {:.3}s ({:.0} ops/sec)",
            forward_std,
            n as f64 / forward_std
        );
        println!(
            "    Backward: {:.3}s ({:.0} ops/sec)",
            backward_std,
            n as f64 / backward_std
        );

        // Comparison
        let forward_ratio = forward_bp / forward_std;
        let backward_ratio = backward_bp / backward_std;
        println!("  Ratio (BPlusTree/std):");
        println!(
            "    Forward:  {:.2}x {}",
            forward_ratio,
            if forward_ratio < 1.0 {
                "(faster)"
            } else {
                "(slower)"
            }
        );
        println!(
            "    Backward: {:.2}x {}",
            backward_ratio,
            if backward_ratio < 1.0 {
                "(faster)"
            } else {
                "(slower)"
            }
        );
        println!();
    }
}
