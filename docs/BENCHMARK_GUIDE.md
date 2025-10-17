# Benchmark Guide

## Partial Iteration Benchmark

### Overview

The partial iteration benchmark (`bench_partial_iter`) measures the performance of iterating over a small subset of items in a very large tree. This is a critical operation for database cursors, pagination, and range queries.

### Running the Benchmark

```bash
# Build the benchmark
cargo build --release --bin bench_partial_iter

# Run with default parameters (10M items, iterate 100, capacity 128)
cargo run --release --bin bench_partial_iter

# Run with custom parameters
cargo run --release --bin bench_partial_iter -- [total_size] [iter_count] [capacity]
```

### Parameters

- `total_size`: Total number of items in the tree (default: 10,000,000)
- `iter_count`: Number of items to iterate in each partial iteration (default: 100)
- `capacity`: B+ tree node capacity (default: 128)

### Example Runs

```bash
# Small tree, iterate 100 items
cargo run --release --bin bench_partial_iter -- 100000 100 128

# Medium tree, iterate 1000 items
cargo run --release --bin bench_partial_iter -- 1000000 1000 128

# Large tree, iterate 10 items (shows iteration scaling)
cargo run --release --bin bench_partial_iter -- 10000000 10 128

# Very large tree with default iteration
cargo run --release --bin bench_partial_iter -- 50000000 100 128
```

### Test Scenarios

The benchmark runs five scenarios:

1. **From Beginning**: Iterate first N items from the start of the tree
2. **From Middle**: Start from middle key and iterate N items forward
3. **From End**: Iterate last N items (requires finding the end first)
4. **Random Positions**: Perform 100 separate partial iterations from random keys
5. **Cursor-like**: Perform 1000 tiny iterations of 10 items each (simulates pagination)

### Understanding the Results

The benchmark reports:
- **Total time** for each scenario
- **Per-item cost** (total time / items iterated)
- **Speedup/slowdown** ratio compared to `std::BTreeMap`

Example output:
```
From Beginning       | BPlusTreeMap:    107.806ms (1078059.17ns/item) | std::BTreeMap:      0.007ms (   67.50ns/item)
                       → BPlusTreeMap is 15971.25x SLOWER
```

This means:
- BPlusTreeMap took 107.8ms to iterate 100 items
- std::BTreeMap took 0.007ms (7 microseconds) for the same operation
- BPlusTreeMap is 15,971 times slower

### Key Findings

**Current Status**: With lazy iteration implementation and capacity 128, BPlusTreeMap is now **competitive with or faster than** std::BTreeMap for partial iteration!

- **Random partial iterations**: 1.06x FASTER than std::BTreeMap ✓
- **Cursor-like patterns**: 1.38x FASTER than std::BTreeMap ✓
- **Range queries from middle**: 7x FASTER than std::BTreeMap ✓
- **Proper O(k) scaling**: Time scales linearly with items iterated

See [partial_iteration_results.md](./partial_iteration_results.md) for detailed analysis.

### Interpreting Performance

Good performance characteristics for partial iteration:
- Time should scale linearly with `iter_count`
- Tree size shouldn't significantly impact iteration of small ranges
- Per-item cost should be consistent (~10-100ns per item)

BPlusTreeMap characteristics (with capacity 128):
- ✓ Time scales linearly with `iter_count` (proper O(k) behavior)
- ✓ Competitive per-item cost (9-40ns for random queries)
- ✓ Faster than std for cursor-like patterns (28ns vs 39ns per item)
- ⚠ "From beginning" has initial traversal overhead (avoid full scans)

### Other Benchmarks

#### Insert/Get/Delete Benchmark

```bash
# Full benchmark suite with 1M items
cargo run --release --bin bench_insert

# Custom parameters: [count] [capacity]
cargo run --release --bin bench_insert -- 5000000 32
```

This benchmark measures:
- Sequential insertion
- Random lookups
- Deletion
- Mixed operations
- Full iteration

## Performance Testing Tips

1. **Use release builds**: Always use `--release` for accurate performance measurement
2. **Warm up**: First run may include JIT/optimization overhead
3. **System load**: Close other applications for consistent results
4. **Multiple runs**: Performance can vary 10-20% between runs
5. **Tree size matters**: Test with realistic data sizes for your use case
6. **Capacity tuning**: Try different node capacities (8, 16, 32, 64)

## Profiling

For detailed profiling:

```bash
# macOS
cargo build --release --bin bench_partial_iter
sudo xcrun xctrace record --template 'Time Profiler' --launch ./target/release/bench_partial_iter

# Linux with perf
cargo build --release --bin bench_partial_iter
perf record --call-graph=dwarf ./target/release/bench_partial_iter
perf report

# Flamegraph
cargo install flamegraph
cargo flamegraph --bin bench_partial_iter
```

## Comparing Results

When comparing versions or implementations:

1. Use identical hardware and system load
2. Run each benchmark 3-5 times and take the median
3. Keep tree size and parameters constant
4. **Always use capacity 128** for optimal performance
5. Focus on relative performance (ratios) rather than absolute times
6. Pay attention to both throughput and latency

## Troubleshooting

**Benchmark takes too long**: Reduce `total_size` parameter
```bash
cargo run --release --bin bench_partial_iter -- 1000000 100 128
```

**Out of memory**: Reduce tree size or increase system memory

**Inconsistent results**: Check for background processes, thermal throttling, or swap usage

**Build errors**: Ensure you're on the latest Rust stable version
```bash
rustc --version  # Should be 1.70+
cargo clean
cargo build --release
```
