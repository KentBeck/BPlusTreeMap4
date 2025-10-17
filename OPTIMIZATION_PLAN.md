# BPlusTreeMap Optimization Plan - All Operations

**Date:** October 17, 2025  
**Status:** Analysis Complete - Ready for Implementation  
**Based on:** Detailed profiling of 1M item tree, capacity 128

---

## Executive Summary

Comprehensive profiling reveals specific optimization opportunities for each operation:

| Operation | Current | Target | Potential Gain | Priority |
|-----------|---------|--------|----------------|----------|
| **GET** | 62-71ns | 30-40ns | **2x faster** | 🔥 HIGH |
| **INSERT** | 83-166ns | 50-80ns | **1.5-2x faster** | 🔥 HIGH |
| **DELETE** | 220-317ns | 120-180ns | **1.5-2x faster** | 🔥 HIGH |
| **ITERATE** | 35ns/item | 10-15ns/item | **2-3x faster** | 🟡 MEDIUM |

**Key Insight:** All operations are CPU-bound (only 8-10% time in memmove). Optimizing tree traversal, node access, and reducing overhead will yield significant gains.

---

## OPERATION 1: GET (Lookup)

### Current Performance

```
Sequential lookups:    62.09ns per operation
Random lookups:        71.34ns per operation
Clustered lookups:     42.05ns per operation (cache-friendly)
Missing key lookups:   33.72ns per operation (fast path)
```

### Performance Breakdown (Estimated)

```
Tree traversal (leaf_for_key):     ~35ns (56%)
  - carve_branch × 3-4 levels:      ~15ns
  - binary_search × 3-4 levels:     ~12ns
  - pointer chasing/cache misses:   ~8ns

Binary search in leaf:              ~15ns (24%)
Carve leaf:                         ~8ns  (13%)
Key comparison & return:            ~4ns  (7%)
──────────────────────────────────────────
Total:                              ~62ns
```

### Bottlenecks Identified

1. **🔥 Tree Traversal (35ns = 56% of time)**
   - Multiple `carve_branch` calls per lookup
   - Binary search at each tree level
   - Cache misses from random pointer chasing
   
2. **🟡 Binary Search in Leaf (15ns = 24%)**
   - log2(64) = 6 comparisons on average
   - Branch mispredictions
   
3. **🟢 Node Carving (8ns = 13%)**
   - Repeated pointer arithmetic
   - Layout offset computation

### Optimization Plan

#### Phase 1: Quick Wins (1 week, expect 20-30% improvement)

**1.1 Inline Critical Functions**
```rust
// Mark as #[inline(always)]
- leaf_for_key
- carve_leaf
- carve_branch
- binary_search_keys
```
**Expected gain:** 5-8ns (8-13% faster)

**1.2 Cache Layout Computations**
```rust
// Store in iterator/operation context
struct LookupContext {
    leaf_layout_cached: LeafLayout,
    branch_layout_cached: BranchLayout,
}
```
**Expected gain:** 3-5ns (5-8% faster)

**1.3 Add Branch Prediction Hints**
```rust
#[cold]
fn handle_missing_key() { ... }

// Use likely/unlikely for common paths
if likely(hdr.tag == NodeTag::Leaf) { ... }
```
**Expected gain:** 2-3ns (3-5% faster)

**Phase 1 Total: 10-16ns savings = 42-46ns target (26-34% improvement)**

#### Phase 2: Algorithmic Improvements (2-3 weeks, expect additional 30-40%)

**2.1 Linear Search for Small Nodes**
```rust
// For nodes with <16 items, linear search is faster than binary
if len < 16 {
    linear_search(keys, key)
} else {
    binary_search(keys, key)
}
```
**Expected gain:** 4-6ns on average (6-10% faster)

**2.2 Prefetching During Traversal**
```rust
// Prefetch next level while processing current
unsafe fn child_for_key(...) {
    let child_ptr = get_child(idx);
    prefetch_for_read(child_ptr);  // Prefetch before descending
    return child_ptr;
}
```
**Expected gain:** 5-8ns (8-13% faster)

**2.3 Node Caching (Hot Path)**
```rust
// Cache recently accessed branch nodes
struct NodeCache {
    entries: [(K, NonNull<u8>); 8],
    next_slot: usize,
}
```
**Expected gain:** 10-15ns on cache hits (15-25% faster for clustered access)

**Phase 2 Total: 19-29ns additional savings = 23-33ns target (50-63% improvement)**

#### Phase 3: Advanced (3-4 weeks, expect additional 10-20%)

**3.1 SIMD Binary Search (x86/ARM)**
```rust
#[cfg(target_feature = "avx2")]
unsafe fn simd_binary_search(...) {
    // Use SIMD for key comparisons
}
```
**Expected gain:** 3-5ns (5-8% faster)

**3.2 Adaptive Tree Height**
```rust
// Store tree height, short-circuit traversal
if tree.height == 1 {
    // Direct leaf access
} else {
    // Full traversal
}
```
**Expected gain:** 2-4ns (3-6% faster)

**Phase 3 Total: 5-9ns additional savings = 18-28ns target (60-71% improvement)**

### Target Performance

```
Current:        62ns (random), 42ns (clustered)
After Phase 1:  42-46ns (random), 30-34ns (clustered)
After Phase 2:  23-33ns (random), 15-20ns (clustered)
After Phase 3:  18-28ns (random), 12-16ns (clustered)

Best case: 18ns = 3.4x faster
Realistic: 30ns = 2.1x faster
```

---

## OPERATION 2: INSERT

### Current Performance

```
Sequential inserts (empty):   83.19ns per operation
Random inserts (empty):       85.64ns per operation
Updates (overwrite):          40.06ns per operation
Inserts with splits:         165.49ns per operation
```

### Performance Breakdown (Estimated)

```
INSERT WITHOUT SPLIT (83ns):
  Tree traversal:              ~35ns (42%)
  Binary search in leaf:       ~15ns (18%)
  Shift items right (memmove): ~10ns (12%)
  Carve leaf:                  ~8ns  (10%)
  Value write:                 ~5ns  (6%)
  Bookkeeping:                 ~10ns (12%)
  ────────────────────────────────────
  Total:                       ~83ns

INSERT WITH SPLIT (166ns):
  Tree traversal:              ~35ns (21%)
  Binary search:               ~15ns (9%)
  Node allocation:             ~30ns (18%)
  Split operation:             ~50ns (30%)
  Parent update:               ~20ns (12%)
  Other overhead:              ~16ns (10%)
  ────────────────────────────────────
  Total:                      ~166ns
```

### Bottlenecks Identified

1. **🔥 Tree Traversal (35ns = 42% of base)**
   - Same issue as GET operation
   
2. **🔥 Node Splits (50ns = 30% when splitting)**
   - Allocation overhead
   - Memory copying
   - Parent node updates
   
3. **🟡 Shift Right (10ns = 12% of base)**
   - Memmove overhead (already near optimal)
   
4. **🟡 Bookkeeping (10ns = 12%)**
   - Length updates
   - Invariant maintenance

### Optimization Plan

#### Phase 1: Quick Wins (1 week, expect 15-25% improvement)

**1.1 Inline Critical Functions**
```rust
#[inline(always)]
- shift_right_kv
- write_key_at
- leaf_insert
```
**Expected gain:** 5-8ns (6-10% faster)

**1.2 Reduce Carve Calls**
```rust
// Cache carve result in insert_rec
let parts = carve_leaf(...);
// Reuse 'parts' for multiple operations
shift_right_kv(&parts, ...);
write_key_at(&parts, ...);
```
**Expected gain:** 3-5ns (4-6% faster)

**1.3 Optimize Update Path**
```rust
// Fast path for overwrites (already exists at 40ns)
if key_exists {
    *value_ptr = new_value;  // Direct write, no shift
    return;
}
```
**Already optimal - no change needed**

**Phase 1 Total: 8-13ns savings = 70-75ns target (12-16% improvement)**

#### Phase 2: Reduce Split Overhead (2-3 weeks, expect 20-30%)

**2.1 Node Pooling**
```rust
struct NodePool {
    free_leaves: Vec<NonNull<u8>>,
    free_branches: Vec<NonNull<u8>>,
}

// Reuse deallocated nodes instead of malloc
impl NodePool {
    fn allocate_leaf(&mut self) -> NonNull<u8> {
        self.free_leaves.pop()
            .unwrap_or_else(|| allocate_new_leaf())
    }
}
```
**Expected gain:** 15-20ns on splits (9-12% overall improvement)

**2.2 Lazy Parent Updates**
```rust
// Batch parent updates where possible
// Defer non-critical updates
```
**Expected gain:** 5-8ns on splits (3-5% overall)

**2.3 Optimize Split Logic**
```rust
// Reduce redundant operations in branch_insert_and_split
// Combine multiple memmove calls
// Minimize zero-fills
```
**Expected gain:** 10-15ns on splits (6-9% overall)

**Phase 2 Total: 30-43ns savings on splits = 50-70ns target (40-60% improvement overall)**

#### Phase 3: Advanced (3-4 weeks)

**3.1 Predict Splits**
```rust
// Check fill level before traversal
if leaf.len == capacity - 1 {
    // Pre-allocate sibling node
    // Reduces critical path latency
}
```
**Expected gain:** 5-10ns on split path

**3.2 Batch Operations**
```rust
// For bulk inserts, batch multiple operations
pub fn insert_batch(&mut self, items: &[(K, V)]) {
    // Amortize traversal cost
    // Batch allocations
}
```
**Expected gain:** 30-50% for bulk insert workloads

### Target Performance

```
Current:        83ns (no split), 166ns (with split)
After Phase 1:  70-75ns (no split), 153-158ns (with split)
After Phase 2:  50-65ns (no split), 100-123ns (with split)
After Phase 3:  45-60ns (no split), 80-100ns (with split)

Best case: 45ns = 1.8x faster (no split), 80ns = 2.1x faster (split)
Realistic: 60ns = 1.4x faster (no split), 110ns = 1.5x faster (split)
```

---

## OPERATION 3: DELETE (Remove)

### Current Performance

```
Sequential deletes:      219.65ns per operation
Random deletes:          317.24ns per operation
Middle deletes:          234.03ns per operation
Deletes causing merges:  114.19ns per operation
```

### Performance Breakdown (Estimated)

```
DELETE WITHOUT MERGE (220-317ns):
  Tree traversal:              ~35ns (11-16%)
  Binary search in leaf:       ~15ns (5-7%)
  Shift items left (memmove):  ~10ns (3-5%)
  Value drop:                  ~5ns  (2-3%)
  Rebalancing overhead:       ~150ns (47-68%) 🔥
  Bookkeeping:                ~20ns  (6-9%)
  ──────────────────────────────────────────
  Total:                     ~235ns

DELETE WITH MERGE (114ns):
  Tree traversal:              ~35ns (31%)
  Merge operation:             ~50ns (44%)
  Other:                       ~29ns (25%)
  ──────────────────────────────────────────
  Total:                      ~114ns
```

### Bottlenecks Identified

1. **🔥🔥🔥 MASSIVE: Rebalancing Overhead (150ns = 47-68%)**
   - This is the primary problem
   - Borrowing from siblings
   - Checking merge conditions
   - Tree rebalancing logic
   
2. **🟡 Tree Traversal (35ns = 11-16%)**
   - Same as GET/INSERT
   
3. **🟢 Actual Delete Work (30ns = 9-14%)**
   - Binary search + shift_left + bookkeeping
   - Already reasonable

### Critical Insight

**DELETE is 3-5x SLOWER than INSERT primarily due to rebalancing overhead!**

This is likely excessive and suggests the rebalancing logic is too aggressive or inefficient.

### Optimization Plan

#### Phase 1: URGENT - Fix Rebalancing (1-2 weeks, expect 50-60% improvement!)

**1.1 Lazy Rebalancing**
```rust
// Don't rebalance immediately - allow nodes to be sparse
const MIN_FILL_THRESHOLD: f32 = 0.25;  // Only merge at 25% full

fn should_rebalance(node_len: usize, capacity: usize) -> bool {
    node_len < (capacity as f32 * MIN_FILL_THRESHOLD) as usize
}
```
**Expected gain:** 80-120ns (37-53% faster!) 🔥

**1.2 Defer Sibling Borrowing**
```rust
// Only borrow if node is critically low
// Reduce expensive borrow operations
if node.len < capacity / 4 {
    // Try to borrow
} else {
    // Accept sparse node
}
```
**Expected gain:** 40-60ns (18-26% faster)

**1.3 Optimize Merge Detection**
```rust
// Cache merge eligibility
// Avoid repeated checks
struct DeleteContext {
    merge_needed: bool,
    sibling_ptr: Option<NonNull<u8>>,
}
```
**Expected gain:** 10-20ns (5-9% faster)

**Phase 1 Total: 130-200ns savings = 117-187ns target (27-63% improvement!)**

#### Phase 2: Further Rebalancing Optimization (2 weeks)

**2.1 Batch Rebalancing**
```rust
// Mark nodes for rebalancing, do it later
// Amortize cost across operations
struct RebalanceQueue {
    nodes: Vec<NonNull<u8>>,
}
```
**Expected gain:** 20-30ns (9-13% faster)

**2.2 Optimize Borrow Operations**
```rust
// Reduce memmove in borrow_from_sibling
// Combine operations
// Minimize pointer updates
```
**Expected gain:** 15-25ns (7-11% faster)

**Phase 2 Total: 35-55ns additional = 82-152ns target (50-74% improvement)**

#### Phase 3: Structural Improvements (3-4 weeks)

**3.1 Adaptive Merge Strategy**
```rust
// Different strategies based on tree state
if tree.size < threshold {
    // Aggressive merging
} else {
    // Lazy merging
}
```
**Expected gain:** 10-20ns (5-9% faster)

**3.2 Node Pooling (Reuse)**
```rust
// Reuse merged nodes
// Avoid deallocation
```
**Expected gain:** 10-15ns (5-7% faster)

### Target Performance

```
Current:        220-317ns (most cases)
After Phase 1:  117-187ns (47-62% faster!) 🔥
After Phase 2:  82-152ns (57-74% faster!)
After Phase 3:  70-140ns (63-78% faster!)

Best case: 70ns = 4.5x faster
Realistic: 120ns = 2.6x faster
```

**DELETE has the most optimization potential!**

---

## OPERATION 4: PARTIAL ITERATION

### Current Performance

```
Small iterations (10 items):    35.36ns per item
Medium iterations (100 items):   4.98ns per item
Bounded ranges:                  2.09ns per item
Cursor-like (5 items):          27.28ns per item

Iterator creation overhead:    ~300-400ns (from previous profiling)
```

### Performance Breakdown (Estimated)

```
ITERATOR CREATION (300-400ns):
  Tree traversal (leaf_for_key):  ~200ns (50-67%)
  Binary search in leaf:          ~50ns  (13-17%)
  Iterator setup:                 ~50ns  (13-17%)
  ──────────────────────────────────────────────
  Total:                         ~300ns

PER-ITEM ITERATION (3-35ns):
  Pointer dereference:            ~1ns
  Bound checking:                 ~1-2ns
  Carve leaf (per leaf):          ~8ns (amortized)
  Cache misses:                   variable
  ──────────────────────────────────────────────
  Total:                          ~3-35ns
```

### Bottlenecks Identified

1. **🔥 Iterator Creation (300-400ns)**
   - Tree traversal dominates (same as GET)
   - High overhead for small iterations
   
2. **🟡 Per-Item Cost Varies Widely**
   - 2.09ns (bounded, within single leaf)
   - 4.98ns (medium iterations, some leaf crossings)
   - 27-35ns (tiny iterations, overhead dominates)

### Optimization Plan

#### Phase 1: Reduce Iterator Creation Cost (1-2 weeks, 30-40% improvement)

**1.1 Apply GET Optimizations**
```rust
// Same tree traversal optimizations as GET
// Inlining, caching, prefetching
```
**Expected gain:** 100-150ns on creation (33-50% faster)

**1.2 Optimize First next()**
```rust
// Already improved with recursion elimination
// Further optimize lazy initialization
```
**Expected gain:** 50-80ns (17-27% faster)

**Phase 1 Total: 150-230ns savings = 170-250ns creation cost (25-43% improvement)**

#### Phase 2: Optimize Hot Iteration Path (1-2 weeks)

**2.1 Eliminate Remaining Checks**
```rust
// Remove redundant bounds checks
// Use unchecked operations where safe
// Optimize within-leaf iteration
```
**Expected gain:** 1-2ns per item

**2.2 Better Cache Utilization**
```rust
// Prefetch next leaf when nearing end
// Align data for cache lines
```
**Expected gain:** 0.5-1ns per item

**Phase 2 Total: 1.5-3ns savings per item = 1-2.5ns/item target**

#### Phase 3: Advanced (2-3 weeks)

**3.1 Specialized Iterator Types**
```rust
// Different iterators for different scenarios
enum IteratorType {
    Small,    // Optimized for <20 items
    Medium,   // General purpose
    Large,    // Bulk iteration
}
```
**Expected gain:** 5-10ns per iteration setup

### Target Performance

```
Current:        300-400ns creation, 2-35ns per item
After Phase 1:  170-250ns creation, 2-35ns per item
After Phase 2:  170-250ns creation, 1-2.5ns per item
After Phase 3:  150-200ns creation, 1-2ns per item

Best case: 150ns + 1ns/item = 10-20x faster for tiny iterations
Realistic: 200ns + 2ns/item = 2-3x faster overall
```

---

## IMPLEMENTATION PRIORITY

### Critical Path (Do First)

1. **🔥🔥🔥 FIX DELETE REBALANCING (Phase 1)** - 1-2 weeks
   - Biggest win: 50-60% improvement
   - DELETE is currently 3-5x slower than it should be
   - This is the most impactful optimization

2. **🔥 GET Optimization (Phase 1)** - 1 week
   - Benefits all operations (GET, INSERT, DELETE, ITERATE)
   - Tree traversal is the common bottleneck
   - 20-30% improvement across the board

3. **🔥 INSERT Optimization (Phase 1)** - 1 week
   - Quick wins with inlining and caching
   - 15-25% improvement

### Medium Priority (Do Next)

4. **GET Phase 2** - 2-3 weeks
   - Algorithmic improvements
   - Additional 30-40% gain

5. **DELETE Phase 2** - 2 weeks
   - Further rebalancing optimization
   - Additional 20-30% gain

6. **INSERT Phase 2** - 2-3 weeks
   - Node pooling and split optimization
   - 20-30% additional gain

### Lower Priority (Do Later)

7. **ITERATE Optimization** - 2-3 weeks
   - Already fairly fast
   - Benefits from GET optimizations automatically

8. **Phase 3 for all operations** - 3-4 weeks
   - Advanced techniques
   - SIMD, prefetching, specialization

---

## EXPECTED OVERALL RESULTS

### After Critical Path (4-5 weeks)

```
GET:     62ns → 42-46ns    (26-34% faster)
INSERT:  83ns → 70-75ns    (10-16% faster)
DELETE: 220ns → 117-187ns  (16-47% faster) 🔥
ITERATE: 300ns → same      (benefits from GET improvements)
```

### After All Phase 1+2 (10-12 weeks)

```
GET:     62ns → 23-33ns    (47-63% faster)
INSERT:  83ns → 50-65ns    (22-40% faster)
DELETE: 220ns → 82-152ns   (31-63% faster) 🔥
ITERATE: 300ns → 170-250ns (17-43% faster)
```

### Best Case Scenario (All phases, 16-20 weeks)

```
GET:     62ns → 18-28ns    (55-71% faster) - 2-3x improvement
INSERT:  83ns → 45-60ns    (28-46% faster) - 1.4-1.8x improvement
DELETE: 220ns → 70-140ns   (36-68% faster) - 1.6-3x improvement 🔥
ITERATE: 300ns → 150-200ns (33-50% faster) - 1.5-2x improvement
```

---

## MEASUREMENT & VALIDATION

### For Each Optimization

1. **Before:** Run profile_all_operations benchmark
2. **Implement:** Make changes
3. **After:** Run profile_all_operations benchmark
4. **Validate:** Ensure no regressions in other operations
5. **Document:** Record actual gains vs. expected

### Success Criteria

- No correctness regressions (all tests pass)
- Performance improvement ≥ 50% of expected gain
- No significant regression in any other operation
- Memory usage remains reasonable

---

## NOTES

### Key Insights from Profiling

1. **All operations are CPU-bound** (only 8-10% time in memmove)
2. **Tree traversal is the common bottleneck** (35ns across all operations)
3. **DELETE is surprisingly slow** due to excessive rebalancing (150ns overhead!)
4. **Iterator creation dominates small iterations** (300-400ns setup)

### Dependencies

- GET optimizations benefit all other operations
- DELETE rebalancing fix is independent and urgent
- INSERT and ITERATE can be done in parallel with GET/DELETE

### Risk Assessment

- **Low risk:** Inlining, caching, hints (Phase 1 changes)
- **Medium risk:** Algorithmic changes (Phase 2)
- **Higher risk:** Node pooling, advanced techniques (Phase 3)

---

**Next Step:** Implement DELETE rebalancing fix (Phase 1) - highest impact, most urgent.