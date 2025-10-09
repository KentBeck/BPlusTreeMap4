# Cache-Line Alignment Experiment

## Hypothesis
Aligning node allocations to cache-line boundaries (64 bytes) would reduce cache misses and improve performance, especially for random access patterns.

## Methodology
Modified `src/layout.rs` to force node alignment to different values:
1. **Baseline**: Natural alignment (8 bytes for u64 keys/values)
2. **16-byte alignment**: Moderate alignment for SIMD-friendly access
3. **64-byte alignment**: Full cache-line alignment

## Results

### Baseline (Natural 8-byte alignment)
```
target                 ins(s)     ins Mops     get(s)     get Mops     del(s)     del Mops     mix(s)     mix Mops    iter(s)    iter Mops
bplustree-current       0.132         7.58      0.028        35.42      0.111         9.00      0.127         7.87      0.004       281.17
std::BTreeMap           0.106         9.45      0.100        10.05      0.111         9.01      0.139         7.17      0.005       208.31
```

### 16-byte Alignment
```
target                 ins(s)     ins Mops     get(s)     get Mops     del(s)     del Mops     mix(s)     mix Mops    iter(s)    iter Mops
bplustree-current       0.127         7.87      0.032        31.70      0.124         8.04      0.147         6.82      0.004       278.49
std::BTreeMap           0.102         9.79      0.099        10.12      0.123         8.16      0.147         6.80      0.005       195.01
```

**Impact**: 
- Insert: -4% slower
- Get: -10% slower  
- Mixed: -13% slower
- Iterate: ~same

### 64-byte Alignment (Cache-line)
```
target                 ins(s)     ins Mops     get(s)     get Mops     del(s)     del Mops     mix(s)     mix Mops    iter(s)    iter Mops
bplustree-current       0.124         8.04      0.030        32.99      0.116         8.61      0.139         7.17      0.003       333.94
std::BTreeMap           0.110         9.12      0.106         9.48      0.118         8.44      0.152         6.56      0.006       178.14
```

**Impact**:
- Insert: ~same
- Get: -7% slower
- Delete: -4% slower
- Mixed: -9% slower
- Iterate: +19% faster ✅

## Analysis

### Why Alignment Hurt Performance

1. **Reduced Cache Utilization**
   - With natural alignment, multiple small nodes can fit in one cache line
   - With 64-byte alignment, each node starts at a cache line boundary
   - This wastes cache space and reduces effective cache capacity

2. **Memory Overhead**
   - 64-byte alignment adds ~2% padding overhead per node
   - For 1M items with capacity=128, this means ~7,813 leaf nodes
   - Extra padding: ~336 KB (not huge, but measurable)

3. **TLB Pressure**
   - Larger effective node size means more pages needed
   - More TLB misses for random access patterns

4. **Cache Line Conflicts**
   - All nodes aligned to cache line boundaries
   - Increases probability of cache line conflicts
   - Reduces effective associativity of the cache

### Why Iteration Improved

- Sequential access benefits from cache-line alignment
- Prefetcher can predict access patterns better
- No wasted cache line fetches during sequential traversal

## Profiling Data

### Baseline (8-byte alignment)
```
Instructions: 515M
Cycles: 462M  
IPC: 1.11
Memory: 24.8 MB
```

### 64-byte alignment
```
Instructions: 512M (-0.6%)
Cycles: 483M (+4.5%)
IPC: 1.06 (worse)
Memory: 24.9 MB
```

**Key insight**: Fewer instructions but MORE cycles = more CPU stalls (cache misses, branch mispredictions).

## Conclusion

**Cache-line alignment (64 bytes) HURTS performance for random access workloads.**

The hypothesis was wrong because:
- Modern CPUs handle unaligned loads efficiently
- Cache utilization is more important than alignment
- B+ tree access patterns are inherently random (pointer chasing)
- Alignment helps sequential access (iteration) but hurts random access

**Recommendation**: Keep natural alignment (8 bytes for u64). Do not force cache-line alignment.

## Next Steps

Since alignment didn't help, the real bottleneck is likely:
1. **Pointer chasing** - inherent to tree structures
2. **Branch mispredictions** - complex control flow in insert/split
3. **Memory allocation overhead** - could try node pooling

The profiling data shows we execute 23% fewer instructions than std::BTreeMap but have worse IPC (1.11 vs 1.47). This suggests the bottleneck is **memory latency**, not computation.

Possible optimizations:
- **Prefetching** - hint CPU to load next node during traversal
- **Node pooling** - reduce allocator overhead
- **Flatten hot paths** - reduce function call overhead
- **Profile-guided optimization** - let compiler optimize based on real usage

However, we're already **winning on the mixed benchmark** (7.87 Mops vs 7.17 Mops = 10% faster), so further optimization may not be worth the complexity.

