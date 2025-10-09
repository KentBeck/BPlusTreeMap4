# Prefetch Experiment

## Hypothesis
Adding prefetch hints during tree traversal would reduce cache miss latency and improve performance, especially for random access patterns.

## Implementation
Added prefetch instructions in `child_for_key()` to hint the CPU to load the child node into cache before it's accessed:

```rust
// Prefetch the child node to reduce cache miss latency
if !child_ptr.is_null() {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!(
        "prefetcht0 ({0})",
        in(reg) child_ptr,
        options(nostack, preserves_flags)
    );
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!(
        "prfm pldl1keep, [{0}]",
        in(reg) child_ptr,
        options(nostack, preserves_flags)
    );
}
```

This prefetch happens after the binary search in the branch node determines which child to visit next.

## Results

### Baseline (no prefetching)
```
target                 ins(s)     ins Mops     get(s)     get Mops     del(s)     del Mops     mix(s)     mix Mops    iter(s)    iter Mops
bplustree-current       0.132         7.58      0.028        35.42      0.111         9.00      0.127         7.87      0.004       281.17
std::BTreeMap           0.106         9.45      0.100        10.05      0.111         9.01      0.139         7.17      0.005       208.31
```

### With Prefetching (average of 3 runs)
```
Run 1: bplustree-current  0.136  7.36   0.030  32.92   0.134  7.47   0.132  7.57   0.003  337.10
Run 2: bplustree-current  0.115  8.71   0.035  28.96   0.122  8.18   0.151  6.62   0.003  338.55
Run 3: bplustree-current  0.112  8.95   0.030  33.47   0.120  8.31   0.152  6.58   0.003  303.33

Average:                  0.121  8.34   0.032  31.78   0.125  7.99   0.145  6.92   0.003  326.33
```

### Comparison

| Operation | Baseline | With Prefetch | Change |
|-----------|----------|---------------|---------|
| **Insert** | 7.58 Mops | 8.34 Mops | +10% |
| **Get** | 35.42 Mops | 31.78 Mops | ❌ **-10%** |
| **Delete** | 9.00 Mops | 7.99 Mops | ❌ **-11%** |
| **Mixed** | 7.87 Mops | 6.92 Mops | ❌ **-12%** |
| **Iterate** | 281.17 Mops | 326.33 Mops | +16% |

## Profiling Data

### Baseline
```
Instructions: 515M
Cycles: 462M
IPC: 1.11
```

### With Prefetching
```
Instructions: 513M (-0.4%)
Cycles: 504M (+9%)
IPC: 1.02 (worse)
```

## Analysis

**Prefetching HURT performance by 12% on the mixed benchmark!**

### Why Prefetching Failed

1. **Prefetch Overhead**
   - Each prefetch instruction takes CPU cycles
   - For small trees (capacity=128, ~1M items), we traverse ~2-3 levels
   - The overhead of 2-3 prefetch instructions per operation adds up

2. **Cache Pollution**
   - Prefetching brings data into cache that might evict more useful data
   - For random access patterns, prefetched data might not be used immediately
   - This reduces effective cache capacity

3. **Already Good Locality**
   - With capacity=128, nodes are relatively large (~2KB for leaves)
   - The CPU's hardware prefetcher already does a good job
   - Manual prefetching interferes with hardware prefetcher

4. **Prefetch Too Early**
   - We prefetch the child pointer immediately after binary search
   - But we still need to return from the function, check node type, etc.
   - By the time we access the child, the prefetched data might be evicted

5. **Instruction Count vs Cycles**
   - Instructions decreased slightly (-0.4%)
   - But cycles increased significantly (+9%)
   - This means the prefetch instructions are causing pipeline stalls

### Why Insert Improved Slightly

The 10% insert improvement is likely measurement noise or variance. Looking at the individual runs:
- Run 1: 7.36 Mops (worse than baseline)
- Run 2: 8.71 Mops (better than baseline)
- Run 3: 8.95 Mops (better than baseline)

The variance is too high to conclude prefetching helps inserts.

### Why Iteration Improved

Sequential access benefits from prefetching because:
- Predictable access pattern (next sibling pointer)
- Prefetched data is used immediately
- No cache pollution (we're going to access it anyway)

However, iteration is already very fast (281 Mops baseline), so the 16% improvement doesn't matter much for real workloads.

## Conclusion

**Prefetching HURTS performance for random access workloads.**

The hypothesis was wrong because:
- Modern CPUs have sophisticated hardware prefetchers
- Manual prefetching adds overhead without benefit
- Cache pollution reduces effective cache capacity
- The tree is shallow enough that prefetch overhead dominates

**Recommendation**: Do NOT add prefetch instructions. Trust the hardware prefetcher.

## Lessons Learned

1. **Hardware prefetchers are good** - Modern CPUs (especially Apple Silicon) have excellent hardware prefetchers that adapt to access patterns

2. **Measure, don't guess** - Intuition about performance is often wrong. Always measure.

3. **Overhead matters** - Even "free" instructions like prefetch have cost (pipeline slots, cache pollution)

4. **Tree depth matters** - For shallow trees (2-3 levels), prefetch overhead dominates any benefit

5. **IPC is key** - Fewer instructions but more cycles = worse performance

## Alternative Approaches

If we wanted to improve cache performance, better approaches would be:

1. **Reduce tree depth** - Use larger capacity (but this hurts split/merge performance)

2. **Improve locality** - Allocate related nodes together (slab allocator)

3. **Reduce pointer chasing** - Flatten hot paths, inline small nodes

4. **Better data layout** - Pack frequently-accessed fields together

5. **Profile-guided optimization** - Let the compiler optimize based on real usage

However, we're already **winning on the mixed benchmark** (7.87 Mops vs 7.17 Mops = 10% faster than std::BTreeMap), so further optimization may not be worth the complexity.

## Final Verdict

**Prefetching: ❌ REJECTED**

Keep the code simple. Trust the hardware.

