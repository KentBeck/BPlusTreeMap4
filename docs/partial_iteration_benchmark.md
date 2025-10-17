# Partial Iteration Benchmark Results

## Overview

This document presents benchmark results comparing partial iteration performance between `BPlusTreeMap` and `std::collections::BTreeMap`. Partial iteration refers to iterating over a small subset of items in a very large tree, which is a common use case for database cursors, pagination, and range queries.

## Key Findings

**The current `BPlusTreeMap` implementation is significantly slower than `std::BTreeMap` for partial iteration**, with performance degrading as the tree size increases. The root cause is that `BPlusTreeMap::range()` eagerly collects all matching items into a `Vec` before returning an iterator, even when only a few items are needed.

## Benchmark Methodology

The benchmark measures five scenarios:
1. **From Beginning**: Iterate the first N items from the start of the tree
2. **From Middle**: Start iteration from the middle key and iterate N items
3. **From End**: Iterate the last N items
4. **Random Positions**: Perform 100 separate partial iterations from random positions
5. **Cursor-like**: Perform 1000 tiny iterations of 10 items each (simulating cursor/pagination behavior)

## Results: Large Tree (10M items, iterate 100)

```
Total items in tree: 10,000,000
Items to iterate: 100
B+ tree capacity: 16

Scenario                BPlusTreeMap     std::BTreeMap    Slowdown
------------------------------------------------------------------------
From Beginning          107.8ms          0.007ms          15,971x slower
From Middle             42.3ms           0.011ms          3,730x slower
From End                87.1ms           28.8ms           3.0x slower
Random Positions (100)  5,022ms          0.144ms          34,946x slower
Cursor-like (1000×10)   48,071ms         0.490ms          98,196x slower
```

### Analysis

- **From Beginning**: BPlusTreeMap takes ~108ms vs 7μs for std - collecting 10M items into a Vec when only 100 are needed
- **From Middle**: Better than beginning (~42ms) but still 3,730x slower - must traverse tree to find starting point, then collect
- **From End**: Least terrible (3x slower) - std::BTreeMap also needs to scan to find last items
- **Random Positions**: Catastrophically slow - each of 100 iterations collects all items from that point to the end
- **Cursor-like**: Worst case - 1000 separate iterations means 1000 Vec allocations and collections

## Results: Medium Tree (100K items, iterate 100)

```
Total items in tree: 100,000
Items to iterate: 100
B+ tree capacity: 16

Scenario                BPlusTreeMap     std::BTreeMap    Slowdown
------------------------------------------------------------------------
From Beginning          0.91ms           0.001ms          949x slower
From Middle             0.14ms           0.001ms          102x slower
From End                0.27ms           0.23ms           1.2x slower
Random Positions (100)  15.2ms           0.08ms           191x slower
Cursor-like (1000×10)   119.8ms          0.24ms           500x slower
```

### Analysis

Even with a 100x smaller tree, the performance gap remains enormous:
- Still collecting 100K items when only 100 are needed
- The slowdown factor is proportionally better, but absolute performance is still poor
- std::BTreeMap maintains consistent per-item iteration cost regardless of tree size

## Per-Item Iteration Cost

### Large Tree (10M items):
- **BPlusTreeMap**: 1,078μs per item (from beginning) to 4,807μs per item (cursor-like)
- **std::BTreeMap**: 68ns per item (from beginning) to 49ns per item (cursor-like)

### Medium Tree (100K items):
- **BPlusTreeMap**: 9μs per item (from beginning) to 12μs per item (cursor-like)
- **std::BTreeMap**: 10ns per item (from beginning) to 24ns per item (cursor-like)

## Root Cause

The `BPlusTreeMap::range()` implementation calls `collect_range_bounds()`, which:

```rust
pub fn range<R: RangeBounds<K>>(&self, r: R) -> Items<'_, K, V> {
    Items {
        inner: self
            .collect_range_bounds(r.start_bound(), r.end_bound())
            .into_iter(),
    }
}
```

This eagerly collects **all items in the range** into a `Vec` before returning an iterator. For unbounded or large ranges, this means:
- Allocating a massive Vec
- Traversing and collecting potentially millions of items
- High memory usage
- All work done upfront, even if caller only needs a few items

In contrast, `std::BTreeMap` uses a lazy iterator that:
- Maintains only a stack of tree positions
- Yields items on-demand as `next()` is called
- Uses minimal memory regardless of range size
- Does minimal work if iteration stops early

## Recommendations

To achieve competitive partial iteration performance, `BPlusTreeMap` needs a **lazy iterator implementation**:

1. **State-based Iterator**: Store current leaf node pointer and index, advance on `next()`
2. **Bounded Range Support**: Stop iteration when reaching the end bound
3. **Memory Efficiency**: No Vec allocation or pre-collection
4. **Early Termination**: When caller drops iterator or stops calling `next()`, no wasted work

### Suggested Iterator Structure

```rust
pub struct LazyItems<'a, K, V> {
    tree: &'a BPlusTreeMap<K, V>,
    current_leaf: Option<NonNull<u8>>,
    current_index: usize,
    end_bound: Bound<K>,
}

impl<'a, K: Ord, V> Iterator for LazyItems<'a, K, V> {
    type Item = (&'a K, &'a V);
    
    fn next(&mut self) -> Option<Self::Item> {
        // Yield current item and advance to next
        // Stop when end_bound is reached
        // Follow leaf->next pointers as needed
    }
}
```

## Use Cases Affected

Partial iteration is critical for:
- **Database cursors**: Fetch small batches from large result sets
- **Pagination**: Display page N of results without loading all data
- **Range queries**: Find items in a specific key range
- **Incremental processing**: Process tree items with the ability to pause/resume
- **Monitoring/debugging**: Peek at tree contents without full traversal

With the current implementation, `BPlusTreeMap` is unsuitable for these use cases when the tree is large.

## Iteration Count Scaling Analysis

Testing with a 1M item tree while varying the number of items to iterate reveals the issue:

```
Tree size: 1,000,000 items

Items to Iterate    BPlusTreeMap Time    std::BTreeMap Time    Per-Item Cost
-----------------------------------------------------------------------------
10                  11.2ms               0.004ms               1,117μs vs 400ns
100                 9.5ms                0.004ms               95μs vs 45ns
1,000               9.1ms                0.007ms               9μs vs 7ns
10,000              9.2ms                0.031ms               923ns vs 3ns
```

### Key Observation

**BPlusTreeMap time is nearly constant (~9-11ms) regardless of iteration count!**

This definitively proves the eager collection behavior:
- Whether iterating 10 or 10,000 items, it takes ~9ms
- The 9ms is the time to collect ALL 1M items into a Vec
- Actual iteration over the Vec is negligible
- std::BTreeMap scales linearly with iteration count (as expected for lazy iteration)

### Cost Breakdown

For a 1M item tree:
- **BPlusTreeMap**: ~9ms fixed cost (Vec collection) + negligible iteration
- **std::BTreeMap**: ~3-400ns per item iterated (no upfront cost)

When iterating only 10 items:
- BPlusTreeMap does 1M units of work (collects everything)
- std::BTreeMap does 10 units of work (iterates only what's needed)
- **Waste factor: 100,000x more work than necessary**

## Conclusion

The current `BPlusTreeMap` implementation has a **critical performance issue for partial iteration**. While full iteration might be acceptable for small trees, the eager collection strategy makes it **98,000x slower** than `std::BTreeMap` for cursor-like access patterns on large trees.

The constant-time behavior regardless of iteration count proves that BPlusTreeMap performs O(total_tree_size) work even for O(1) iterations, while std::BTreeMap correctly performs O(iterations) work.

**Priority**: This should be considered a high-priority issue to fix before production use, as partial iteration is a fundamental tree operation.