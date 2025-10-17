# Partial Iteration Performance Hotspots Analysis

**Date:** October 17, 2025  
**Status:** Analysis Complete  
**Tree Size:** 10M items, Capacity: 128

---

## Executive Summary

BPlusTreeMap's partial iteration is already **1.1-3x faster than std::BTreeMap** in realistic scenarios. However, micro-benchmark analysis reveals specific hotspots that could yield further improvements:

**Key Finding:** First `next()` call has **704ns overhead** due to lazy initialization, while subsequent items cost only **2.86-6.13ns each**.

---

## Micro-Benchmark Results

### Cost Breakdown (per operation)

| Operation | Cost (ns) | % of Total | Optimization Potential |
|-----------|-----------|------------|------------------------|
| Iterator creation | 3.36 | 0.5% | ✓ Already optimal |
| First next() (initialization) | 704.12 | 94.8% | 🔥 **HOT SPOT #1** |
| Subsequent next() (within leaf) | 2.86 | 0.4% | ✓ Already optimal |
| Subsequent next() (cross leaf) | 4.18 | 0.6% | Minor opportunity |
| Per-item (100 item batch) | 6.13 | 0.8% | ✓ Good performance |
| Per-item (10 item batch) | 20.39 | 2.7% | Amortization issue |
| Bounded range check | 9.23 | 1.2% | Minor opportunity |

### Key Observations

1. **First next() dominates small iterations**: 704ns setup for 10 items = 70ns/item overhead
2. **Amortization works well**: For 100+ items, overhead becomes negligible (~7ns/item)
3. **Leaf traversal is efficient**: Only 1.31ns extra cost for crossing leaf boundaries
4. **Within-leaf iteration is excellent**: 2.86ns per item (memory bandwidth limited)

---

## Identified Hotspots

### 🔥 HOT SPOT #1: Lazy Initialization (704ns)

**Location:** `src/iterate.rs` lines 46-104 (inside first `next()` call)

**What happens:**
```rust
if !*initialized {
    *initialized = true;
    let is_excluded = matches!(start_bound, Bound::Excluded(_));
    match start_bound {
        Bound::Unbounded => {
            *front_leaf = tree.leftmost_leaf();  // Tree traversal
            *front_idx = 0;
        }
        Bound::Included(k) | Bound::Excluded(k) => {
            let leaf_opt = tree.leaf_for_key(k);  // Binary search down tree
            if let Some(leaf) = leaf_opt {
                unsafe {
                    let parts = layout::carve_leaf::<K, V>(leaf, &tree.leaf_layout);
                    let len = (*parts.hdr).len as usize;
                    let keys = core::slice::from_raw_parts(
                        parts.keys_ptr as *const K,
                        len,
                    );
                    match keys.binary_search(k) {  // Binary search in leaf
                        // ... positioning logic
                    }
                }
            }
        }
    }
}
```

**Cost breakdown (estimated):**
- `leaf_for_key()`: ~400-500ns (tree traversal with O(log n) branches)
- `carve_leaf()`: ~50-100ns (pointer arithmetic + memory access)
- `binary_search()`: ~50-100ns (binary search in leaf array)
- Positioning logic: ~50-100ns (conditional branches)

**Impact:**
- **Critical for small iterations** (10-50 items): 70-20ns/item overhead
- **Minor for medium iterations** (100+ items): <7ns/item overhead
- **Affects cursor-like workloads most** (1000 iterations × 10 items each)

**Optimization opportunities:**
1. ✅ **Already lazy** - deferred until first next()
2. 🔥 **Cache starting leaf** - store last accessed leaf/position
3. 🔥 **Hint-based positioning** - accept optional starting position hint
4. 🟡 **Speculative prefetch** - prefetch likely next leaf during init
5. 🟡 **Reduce carve_leaf calls** - cache layout computation

---

### 🔥 HOT SPOT #2: Tree Traversal in leaf_for_key() (~400-500ns)

**Location:** `src/common.rs` lines 140-158

**What happens:**
```rust
pub(crate) fn leaf_for_key(&self, key: &K) -> Option<NonNull<u8>> {
    let mut cur = self.root?;
    unsafe {
        loop {
            let hdr = &*(cur.as_ptr() as *const NodeHdr);
            match hdr.tag {
                NodeTag::Leaf => return Some(cur),
                NodeTag::Branch => {
                    if let Some((child, _)) = self.child_for_key(cur, key) {
                        cur = child;
                    } else {
                        return None;
                    }
                }
            }
        }
    }
}
```

**Cost factors:**
- Tree height for 10M items, capacity 128: ~3-4 levels
- Per-level cost: ~100-150ns
  - carve_branch(): ~30ns
  - binary_search_keys(): ~50ns
  - Pointer chase: ~20-50ns (cache miss potential)

**Impact:**
- Called once per range iterator creation
- 57% of initialization cost
- Cache misses dominate (pointer chasing through tree levels)

**Optimization opportunities:**
1. 🔥 **Iterator position caching** - reuse last position for nearby queries
2. 🟡 **Prefetch next level** - issue prefetch during binary search
3. 🟡 **Reduce tree height** - increase capacity (but trades off insert performance)
4. 🟢 **Branch prediction hints** - add likely() annotations for common paths

---

### 🟡 HOT SPOT #3: Binary Search in Leaf (~50-100ns)

**Location:** `src/iterate.rs` lines 65-88

**What happens:**
```rust
let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
match keys.binary_search(k) {
    Ok(i) => {
        let idx = if is_excluded { i + 1 } else { i };
        if idx >= len {
            // Move to next leaf
            let next_ptr = *parts.next_ptr;
            *front_leaf = NonNull::new(next_ptr);
            *front_idx = 0;
        } else {
            *front_leaf = Some(leaf);
            *front_idx = idx;
        }
    }
    Err(i) => { /* similar */ }
}
```

**Cost factors:**
- Binary search in 128-item array: log2(128) = 7 comparisons
- Key comparison cost: depends on K type (for u64: ~5-10ns)
- Branch mispredictions: ~5-10ns
- Positioning logic: ~10-20ns

**Impact:**
- 7-14% of initialization cost
- Only happens during initialization

**Optimization opportunities:**
1. 🟡 **Linear search for small leaves** - below threshold (~32 items), linear is faster
2. 🟡 **SIMD search** - for primitive types (u32, u64)
3. 🟡 **Interpolation search** - if keys are uniformly distributed
4. 🟢 **Branchless binary search** - reduce branch mispredictions

---

### 🟡 HOT SPOT #4: Bound Checking in next() (~2-5ns per item)

**Location:** `src/iterate.rs` lines 110-122

**What happens:**
```rust
let k = &*(parts.keys_ptr.add(*front_idx) as *const K);

// Check end bound
let within_bound = match end_bound {
    Bound::Unbounded => true,
    Bound::Included(e) => k <= e,
    Bound::Excluded(e) => k < e,
};

if !within_bound {
    *front_leaf = None;
    *remaining = 0;
    return None;
}
```

**Cost factors:**
- Match on end_bound: ~1ns (branch)
- Key comparison (if bounded): ~5-10ns (may involve comparison trait)
- Conditional check: ~1-2ns

**Impact:**
- 2-5ns overhead per item for bounded ranges
- Only affects bounded range queries (not open-ended ranges)
- Shows up in Test 7 (9.23ns vs 2.86ns baseline)

**Optimization opportunities:**
1. 🟡 **Separate unbounded path** - specialized iterator variant
2. 🟡 **Hoist bound check** - check once per leaf, not per item
3. 🟢 **Branch prediction hints** - annotate common path (within bounds)

---

### 🟢 HOT SPOT #5: carve_leaf() Calls (~30-50ns per call)

**Location:** `src/layout.rs` lines 318-334

**What happens:**
```rust
pub unsafe fn carve_leaf<K, V>(base: NonNull<u8>, layout: &LeafLayout) -> LeafParts<K, V> {
    let p = base.as_ptr();
    let hdr = p as *mut NodeHdr;
    let next_ptr = p.add(layout.next_off) as *mut *mut u8;
    let prev_ptr = layout.prev_off.map(|off| p.add(off) as *mut *mut u8);
    let keys_ptr = p.add(layout.keys_off) as *mut MaybeUninit<K>;
    let vals_ptr = p.add(layout.vals_off) as *mut MaybeUninit<V>;
    LeafParts { hdr, next_ptr, prev_ptr, keys_ptr, vals_ptr }
}
```

**Cost factors:**
- Multiple pointer arithmetic operations: ~5-10ns
- Struct construction: ~5ns
- Memory loads (for offsets): ~10-20ns (cache dependent)

**Impact:**
- Called once during initialization
- Called once per leaf traversal during iteration
- For 200 items across ~2 leaves: ~2 calls = ~100ns total

**Optimization opportunities:**
1. 🟢 **Inline aggressively** - already has #[inline]
2. 🟢 **Cache leaf parts** - store in iterator state
3. 🟢 **Compute offsets once** - avoid repeated layout.field access

---

### 🟢 HOT SPOT #6: Leaf Boundary Crossing (~1.31ns extra per crossing)

**Location:** `src/iterate.rs` lines 124-136

**What happens:**
```rust
// Move to next leaf
let next_ptr = *parts.next_ptr;
if next_ptr.is_null() {
    *front_leaf = None;
    *remaining = 0;
    return None;
}

*front_leaf = NonNull::new(next_ptr);
*front_idx = 0;
self.next()  // Recursive call
```

**Cost factors:**
- Pointer dereference: ~5-10ns (potential cache miss)
- Null check: ~1ns
- Recursive call overhead: ~5-10ns
- carve_leaf() on new leaf: ~30-50ns (happens in recursive next())

**Impact:**
- Minimal: only 1.31ns extra per boundary (4.18ns - 2.86ns)
- Actually very efficient!
- For 200 items with capacity 128: ~1-2 crossings

**Optimization opportunities:**
1. ✅ **Already optimal** - simple pointer chase
2. 🟢 **Prefetch next leaf** - issue prefetch before finishing current leaf
3. 🟢 **Eliminate recursion** - use loop instead of tail recursion

---

## Performance Potential Analysis

### Current Performance vs std::BTreeMap

| Scenario | BPlusTree | std::BTree | Speedup |
|----------|-----------|------------|---------|
| From Middle (100 items) | 0.005ms | 0.014ms | **2.6x faster** |
| Random Positions (10K items) | 0.489ms | 0.692ms | **1.4x faster** |
| Cursor-like (10K items) | 2.063ms | 2.347ms | **1.1x faster** |

### Optimization Impact Estimates

If we optimize the key hotspots:

#### Scenario 1: Cache Starting Positions (Iterator Reuse)
**Target:** Reduce 704ns initialization to ~50-100ns for nearby queries

**Technique:** Cache last N accessed leaf positions in a small array
```rust
struct IteratorCache {
    entries: [(K, NonNull<u8>, usize); 8],  // 8 cache entries
    next_slot: usize,
}
```

**Expected gains:**
- Cursor-like workload: **3-5x faster** (eliminate most initialization)
- Random positions: **1.5-2x faster** (50% cache hit rate)
- Sequential scans: **Minimal** (already efficient)

**Implementation cost:** Medium (cache invalidation on mutations)

---

#### Scenario 2: Separate Unbounded Iterator
**Target:** Eliminate bound checking for open-ended ranges

**Technique:** Specialized iterator type for unbounded ranges
```rust
impl Iterator for UnboundedItems<'a, K, V> {
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        // No bound checking - simplified hot path
    }
}
```

**Expected gains:**
- Unbounded ranges: **1.2-1.5x faster** (eliminate 2-5ns per item)
- Bounded ranges: **No change** (uses different code path)

**Implementation cost:** Low (code duplication, API addition)

---

#### Scenario 3: Prefetching
**Target:** Reduce cache miss penalties during tree traversal

**Technique:** Issue prefetch hints during traversal
```rust
#[cfg(target_arch = "x86_64")]
unsafe {
    core::arch::x86_64::_mm_prefetch(
        next_ptr as *const i8,
        core::arch::x86_64::_MM_HINT_T0
    );
}
```

**Expected gains:**
- Deep tree traversal: **1.1-1.2x faster** (reduce cache miss latency)
- Shallow trees: **Minimal** (already cache-friendly)

**Implementation cost:** Medium (arch-specific, testing required)

---

#### Scenario 4: Eliminate Recursion in Leaf Crossing
**Target:** Remove recursive call overhead in next()

**Technique:** Replace tail recursion with loop
```rust
loop {
    if *front_idx < len {
        // Return item
        return Some((k, v));
    }
    
    // Move to next leaf
    let next_ptr = *parts.next_ptr;
    if next_ptr.is_null() {
        return None;
    }
    
    *front_leaf = NonNull::new(next_ptr);
    *front_idx = 0;
    // Continue loop instead of recursive call
}
```

**Expected gains:**
- Cross-leaf iteration: **1.05-1.1x faster** (eliminate call overhead)

**Implementation cost:** Low (straightforward refactoring)

---

## Prioritized Optimization Recommendations

### 🔥 High Priority (Significant Impact)

1. **Iterator Position Caching**
   - Impact: 3-5x faster for cursor-like workloads
   - Complexity: Medium
   - Trade-off: Cache invalidation on mutations
   - Estimated effort: 2-3 days

2. **Eliminate Recursion in next()**
   - Impact: 5-10% faster for cross-leaf iteration
   - Complexity: Low
   - Trade-off: None (pure improvement)
   - Estimated effort: 4-8 hours

### 🟡 Medium Priority (Moderate Impact)

3. **Specialized Unbounded Iterator**
   - Impact: 20-50% faster for unbounded ranges
   - Complexity: Low
   - Trade-off: Code duplication
   - Estimated effort: 1-2 days

4. **Hoist Bound Checking to Per-Leaf**
   - Impact: 10-20% faster for bounded ranges
   - Complexity: Medium
   - Trade-off: More complex logic
   - Estimated effort: 1-2 days

### 🟢 Low Priority (Marginal Impact)

5. **Prefetching**
   - Impact: 10-20% faster for deep trees
   - Complexity: Medium-High
   - Trade-off: Architecture-specific code
   - Estimated effort: 2-3 days

6. **Cache leaf_parts in Iterator**
   - Impact: 5-10% reduction in carve_leaf overhead
   - Complexity: Low
   - Trade-off: Larger iterator struct
   - Estimated effort: 4-8 hours

7. **SIMD Binary Search**
   - Impact: 10-20% faster initialization
   - Complexity: High
   - Trade-off: Type constraints, portability
   - Estimated effort: 3-5 days

---

## Realistic Performance Targets

### Conservative Estimates (High Priority Optimizations)

| Scenario | Current | Target | Expected Improvement |
|----------|---------|--------|---------------------|
| From Middle | 2.6x faster | **3.5x faster** | +35% |
| Random Positions | 1.4x faster | **2.0x faster** | +43% |
| Cursor-like | 1.1x faster | **4.0x faster** | +264% |

### Aggressive Estimates (All Optimizations)

| Scenario | Current | Target | Expected Improvement |
|----------|---------|--------|---------------------|
| From Middle | 2.6x faster | **4.5x faster** | +73% |
| Random Positions | 1.4x faster | **3.0x faster** | +114% |
| Cursor-like | 1.1x faster | **5.5x faster** | +400% |

---

## Conclusion

**Current State:** BPlusTreeMap is already production-ready and faster than std::BTreeMap.

**Optimization Potential:** Significant gains possible, especially for cursor-like workloads.

**Low-Hanging Fruit:**
1. Eliminate recursion in next() - Easy win, ~5-10% improvement
2. Add specialized unbounded iterator - Easy win, ~20-50% for unbounded ranges
3. Iterator position caching - Medium effort, 3-5x improvement for cursors

**Recommended Next Steps:**
1. Implement recursion elimination (quick win)
2. Benchmark against std::BTreeMap again
3. If competitive advantage desired, implement iterator caching
4. Consider specialized iterator types for different use cases

**Bottom Line:** Even without optimizations, BPlusTreeMap is excellent. With targeted optimizations, it could be **3-5x faster than std::BTreeMap** for cursor-like workloads.