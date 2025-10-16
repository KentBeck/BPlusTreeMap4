# Range Iteration Optimization Proposal

## Problem Statement

Current bounded range iteration is **4-5.5x slower** than std::BTreeMap:
- BPlusTreeMap: 3.26-7.15ns per element
- std::BTreeMap: 1.30-1.57ns per element

**Root cause:** Repeated overhead in the hot iteration loop.

## Optimization Phases

### Phase 1: Cache Leaf Parts (HIGH PRIORITY)

**Impact:** 2-3x speedup
**Complexity:** Medium
**Risk:** Low

#### Problem

`carve_leaf()` is called on EVERY iteration:
```rust
// Current code - executed per element
let parts = layout::carve_leaf::<K, V>(leaf, &tree.leaf_layout);
let len = (*parts.hdr).len as usize;
let k = &*(parts.keys_ptr.add(*front_idx) as *const K);
let v = &*(parts.vals_ptr.add(*front_idx) as *const V);
```

This computes 5 pointers from base + offsets **every time**, costing 1-2ns per element.

#### Solution

Cache the computed pointers and only recompute when moving to a new leaf:

```rust
pub enum ItemsInner<'a, K, V> {
    Lazy {
        tree: &'a BPlusTreeMap<K, V>,
        front_leaf: Option<NonNull<u8>>,
        front_idx: usize,
        
        // NEW: Cached leaf parts
        cached_keys_ptr: *const K,
        cached_vals_ptr: *const V,
        cached_len: usize,
        cached_next_ptr: *mut u8,
        
        back_leaf: Option<NonNull<u8>>,
        back_idx: usize,
        remaining: usize,
        start_bound: Bound<K>,
        end_bound: Bound<K>,
        initialized: bool,
    },
    Vec { /* ... */ },
}
```

#### Implementation

```rust
fn next(&mut self) -> Option<Self::Item> {
    match &mut self.inner {
        ItemsInner::Lazy {
            tree,
            front_leaf,
            front_idx,
            cached_keys_ptr,
            cached_vals_ptr,
            cached_len,
            cached_next_ptr,
            end_bound,
            initialized,
            ..
        } => {
            // Lazy initialization
            if !*initialized {
                *initialized = true;
                // ... find start position ...
                
                // Cache leaf parts
                let leaf = (*front_leaf)?;
                let parts = layout::carve_leaf::<K, V>(leaf, &tree.leaf_layout);
                *cached_keys_ptr = parts.keys_ptr as *const K;
                *cached_vals_ptr = parts.vals_ptr as *const V;
                *cached_len = (*parts.hdr).len as usize;
                *cached_next_ptr = *parts.next_ptr;
            }
            
            // Fast path: use cached pointers
            if *front_idx < *cached_len {
                let k = unsafe { &**cached_keys_ptr.add(*front_idx) };
                
                // Check end bound
                let within_bound = match end_bound {
                    Bound::Unbounded => true,
                    Bound::Included(e) => k <= e,
                    Bound::Excluded(e) => k < e,
                };
                
                if !within_bound {
                    *front_leaf = None;
                    return None;
                }
                
                let v = unsafe { &**cached_vals_ptr.add(*front_idx) };
                *front_idx += 1;
                return Some((k, v));
            }
            
            // Move to next leaf - recompute cache
            if cached_next_ptr.is_null() {
                *front_leaf = None;
                return None;
            }
            
            *front_leaf = NonNull::new(*cached_next_ptr);
            *front_idx = 0;
            
            let leaf = front_leaf.unwrap();
            let parts = layout::carve_leaf::<K, V>(leaf, &tree.leaf_layout);
            *cached_keys_ptr = parts.keys_ptr as *const K;
            *cached_vals_ptr = parts.vals_ptr as *const V;
            *cached_len = (*parts.hdr).len as usize;
            *cached_next_ptr = *parts.next_ptr;
            
            self.next()
        }
        ItemsInner::Vec { inner } => inner.next(),
    }
}
```

#### Expected Results

- **Before:** 7.15ns per element (small ranges)
- **After:** 4-5ns per element
- **Speedup:** 1.4-1.8x

**Savings:** 1-2ns per element from eliminating repeated `carve_leaf()` calls.

---

### Phase 2: Eliminate Inner-Loop Bound Checking (HIGH PRIORITY)

**Impact:** 1.5-2x additional speedup
**Complexity:** High
**Risk:** Medium (correctness critical)

#### Problem

Bound checking happens on EVERY element:
```rust
// Executed per element
let within_bound = match end_bound {
    Bound::Unbounded => true,
    Bound::Included(e) => k <= e,
    Bound::Excluded(e) => k < e,
};
```

This costs 1-2ns per element due to match + comparison + branch.

#### Solution

Check bounds only at leaf boundaries, not per-element:

```rust
fn next(&mut self) -> Option<Self::Item> {
    // ... initialization ...
    
    // Check if we need to validate bounds
    let need_bound_check = !matches!(end_bound, Bound::Unbounded);
    
    if *front_idx < *cached_len {
        // Fast path: no bound checking within leaf
        if !need_bound_check {
            // Unbounded: just iterate
            let k = unsafe { &**cached_keys_ptr.add(*front_idx) };
            let v = unsafe { &**cached_vals_ptr.add(*front_idx) };
            *front_idx += 1;
            return Some((k, v));
        }
        
        // Bounded: check if entire remaining leaf is in range
        if *front_idx == 0 {
            // First element in leaf: check last element
            let last_key = unsafe { &**cached_keys_ptr.add(*cached_len - 1) };
            let leaf_in_range = match end_bound {
                Bound::Included(e) => last_key <= e,
                Bound::Excluded(e) => last_key < e,
                Bound::Unbounded => true,
            };
            
            if leaf_in_range {
                // Entire leaf is in range: iterate without checking
                let k = unsafe { &**cached_keys_ptr.add(*front_idx) };
                let v = unsafe { &**cached_vals_ptr.add(*front_idx) };
                *front_idx += 1;
                return Some((k, v));
            }
        }
        
        // Slow path: check this element
        let k = unsafe { &**cached_keys_ptr.add(*front_idx) };
        let within_bound = match end_bound {
            Bound::Unbounded => true,
            Bound::Included(e) => k <= e,
            Bound::Excluded(e) => k < e,
        };
        
        if !within_bound {
            *front_leaf = None;
            return None;
        }
        
        let v = unsafe { &**cached_vals_ptr.add(*front_idx) };
        *front_idx += 1;
        return Some((k, v));
    }
    
    // ... move to next leaf ...
}
```

#### Expected Results

- **Before:** 4-5ns per element (after Phase 1)
- **After:** 2-3ns per element
- **Speedup:** 1.5-2x

**Savings:** 1-2ns per element from eliminating most bound checks.

---

### Phase 3: Batch Pointer Iteration (MEDIUM PRIORITY)

**Impact:** 1.2-1.5x additional speedup
**Complexity:** Low
**Risk:** Low

#### Problem

Pointer arithmetic is repeated for keys and values:
```rust
let k = &**cached_keys_ptr.add(*front_idx);
let v = &**cached_vals_ptr.add(*front_idx);
*front_idx += 1;
```

#### Solution

Use raw pointer iteration:

```rust
// Instead of index-based iteration
let mut k_ptr = cached_keys_ptr.add(*front_idx);
let mut v_ptr = cached_vals_ptr.add(*front_idx);
let end_ptr = cached_keys_ptr.add(*cached_len);

while k_ptr < end_ptr {
    let k = &*k_ptr;
    let v = &*v_ptr;
    
    // Bound check if needed
    if need_bound_check && !within_bound(k, end_bound) {
        break;
    }
    
    yield (k, v);
    
    k_ptr = k_ptr.add(1);
    v_ptr = v_ptr.add(1);
}
```

#### Expected Results

- **Before:** 2-3ns per element (after Phase 2)
- **After:** 1.5-2.5ns per element
- **Speedup:** 1.2-1.5x

**Savings:** 0.5-1ns per element from better pointer arithmetic.

---

### Phase 4: Prefetch Next Leaf (LOW PRIORITY)

**Impact:** 1.1-1.2x speedup for sequential access
**Complexity:** Low
**Risk:** Low

#### Problem

Cache miss when moving to next leaf.

#### Solution

Prefetch next leaf when approaching end of current leaf:

```rust
if *front_idx == *cached_len - 4 && !cached_next_ptr.is_null() {
    // Prefetch next leaf header and first few elements
    core::arch::x86_64::_mm_prefetch(
        cached_next_ptr as *const i8,
        core::arch::x86_64::_MM_HINT_T0
    );
}
```

#### Expected Results

- Reduces cache miss penalty when crossing leaf boundaries
- Most beneficial for large ranges with many leaves

---

## Implementation Roadmap

### Milestone 1: Phase 1 (Cache Leaf Parts)

**Timeline:** 1-2 days
**Deliverables:**
- [ ] Update `ItemsInner` enum with cached fields
- [ ] Modify `next()` to use cached pointers
- [ ] Modify `next_back()` similarly
- [ ] Update initialization logic
- [ ] Run all tests
- [ ] Benchmark and verify 1.4-1.8x speedup

**Success Criteria:**
- All tests pass
- Per-element cost: 4-5ns (down from 7.15ns)
- No correctness regressions

### Milestone 2: Phase 2 (Eliminate Bound Checking)

**Timeline:** 2-3 days
**Deliverables:**
- [ ] Implement leaf-boundary bound checking
- [ ] Add fast path for unbounded ranges
- [ ] Add fast path for fully-in-range leaves
- [ ] Extensive testing with various bound types
- [ ] Benchmark and verify 1.5-2x additional speedup

**Success Criteria:**
- All tests pass
- Per-element cost: 2-3ns
- Correct behavior for all bound types

### Milestone 3: Phase 3 (Batch Pointer Iteration)

**Timeline:** 1 day
**Deliverables:**
- [ ] Convert to raw pointer iteration
- [ ] Benchmark and verify 1.2-1.5x additional speedup

**Success Criteria:**
- Per-element cost: 1.5-2.5ns
- Approaching std::BTreeMap performance

### Milestone 4: Phase 4 (Prefetch)

**Timeline:** 1 day
**Deliverables:**
- [ ] Add prefetch hints
- [ ] Benchmark on various workloads

**Success Criteria:**
- Measurable improvement for large ranges
- No regression for small ranges

---

## Expected Final Performance

| Metric | Current | Phase 1 | Phase 2 | Phase 3 | Phase 4 | std::BTreeMap |
|--------|---------|---------|---------|---------|---------|---------------|
| Small (100) | 7.15ns | 4-5ns | 2-3ns | 1.5-2.5ns | 1.5-2.5ns | 1.30ns |
| Medium (10k) | 5.61ns | 3-4ns | 1.5-2.5ns | 1.2-2ns | 1.2-2ns | 1.31ns |
| Large (100k) | 3.26ns | 2-2.5ns | 1.2-1.8ns | 1-1.5ns | 0.9-1.4ns | 1.57ns |

**Target:** Match or beat std::BTreeMap for large ranges, within 2x for small ranges.

---

## Risk Assessment

### Phase 1: Low Risk
- Straightforward caching
- Easy to test
- No algorithmic changes

### Phase 2: Medium Risk
- Complex logic with multiple code paths
- Correctness critical (bound checking)
- Requires extensive testing

### Phase 3: Low Risk
- Simple pointer arithmetic change
- Well-understood optimization

### Phase 4: Low Risk
- Optional optimization
- Can be disabled if problematic

---

## Testing Strategy

1. **Unit tests:** All existing range tests must pass
2. **Fuzz testing:** Random ranges with various bounds
3. **Edge cases:**
   - Empty ranges
   - Single-element ranges
   - Ranges spanning multiple leaves
   - All bound types (Included, Excluded, Unbounded)
4. **Performance regression tests:**
   - Benchmark suite for all range sizes
   - Compare against baseline

---

## Conclusion

The proposed optimizations can reduce per-element iteration cost from **7.15ns to 1.5-2.5ns**, making BPlusTreeMap competitive with std::BTreeMap for range queries.

**Recommended approach:**
1. Start with Phase 1 (cache leaf parts) - biggest win, lowest risk
2. Proceed to Phase 2 if Phase 1 is successful
3. Phases 3 and 4 are optional refinements

**Expected outcome:** 3-5x overall speedup, bringing range iteration performance within 1-2x of std::BTreeMap.
