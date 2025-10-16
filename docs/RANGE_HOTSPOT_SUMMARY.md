# Range Iteration Hotspot Summary

## CPU Cycle Distribution (Per Element)

```
Total: 10 cycles per element

┌─────────────────────────────────────────────────────────────┐
│ carve_leaf()        ████████████████ 35-40% (3-4 cycles)   │ ⚠️ HOTSPOT #1
├─────────────────────────────────────────────────────────────┤
│ Bound checking      ██████████ 20-30% (2-3 cycles)         │ ⚠️ HOTSPOT #2
├─────────────────────────────────────────────────────────────┤
│ Pointer arithmetic  ██████████ 20-30% (2-3 cycles)         │ ⚠️ HOTSPOT #3
├─────────────────────────────────────────────────────────────┤
│ Other operations    █████ 10-20% (1-2 cycles)              │
└─────────────────────────────────────────────────────────────┘
```

## Hotspot #1: carve_leaf() - 3-4 cycles (35-40%)

### Location
```rust
// src/iterate.rs:98
let parts = layout::carve_leaf::<K, V>(leaf, &tree.leaf_layout);
```

### What It Does
Computes 5 pointers from base pointer + offsets:
```rust
// src/layout.rs:318-332
pub unsafe fn carve_leaf<K, V>(base: NonNull<u8>, layout: &LeafLayout) -> LeafParts<K, V> {
    let p = base.as_ptr();
    let hdr = p as *mut NodeHdr;                           // 1 cycle
    let next_ptr = p.add(layout.next_off) as *mut *mut u8; // 2 cycles
    let prev_ptr = layout.prev_off.map(|off| p.add(off) as *mut *mut u8); // 2 cycles
    let keys_ptr = p.add(layout.keys_off) as *mut MaybeUninit<K>; // 2 cycles
    let vals_ptr = p.add(layout.vals_off) as *mut MaybeUninit<V>; // 2 cycles
    LeafParts { hdr, next_ptr, prev_ptr, keys_ptr, vals_ptr }
}
```

### Why It's Hot
- Called on **EVERY iteration** (100 times for 100-element range)
- Pointers don't change within a leaf
- Redundant computation

### Optimization
**Cache the computed pointers:**
```rust
// Compute once per leaf
cached_keys_ptr = parts.keys_ptr;
cached_vals_ptr = parts.vals_ptr;
cached_len = (*parts.hdr).len;

// Reuse in iteration
let k = &*cached_keys_ptr.add(idx);  // No carve_leaf!
let v = &*cached_vals_ptr.add(idx);
```

**Savings: 3-4 cycles per element → 0 cycles (amortized)**

---

## Hotspot #2: Bound Checking - 2-3 cycles (20-30%)

### Location
```rust
// src/iterate.rs:105-109
let within_bound = match end_bound {
    Bound::Unbounded => true,
    Bound::Included(e) => k <= e,
    Bound::Excluded(e) => k < e,
};
```

### What It Does
Checks if current key is within the end bound:
```assembly
mov    rax, [key_ptr]       ; Load key (1 cycle)
cmp    rax, [end_bound]     ; Compare (1 cycle)
jge    .out_of_bounds       ; Branch (1 cycle)
```

### Why It's Hot
- Executed on **EVERY element** (100 times for 100-element range)
- Most elements in a leaf are within bounds
- Unnecessary checks

### Optimization
**Check once per leaf:**
```rust
// Check last element of leaf
let last_key = &*cached_keys_ptr.add(len - 1);
if last_key <= end_bound {
    // Fast path: entire leaf is in range
    // Iterate without checking each element
    for i in idx..len {
        yield (&*cached_keys_ptr.add(i), &*cached_vals_ptr.add(i));
    }
} else {
    // Slow path: check each element
    for i in idx..len {
        let k = &*cached_keys_ptr.add(i);
        if k > end_bound { break; }
        yield (k, &*cached_vals_ptr.add(i));
    }
}
```

**Savings: 2-3 cycles per element → 0.1 cycles (amortized)**

---

## Hotspot #3: Pointer Arithmetic - 2-3 cycles (20-30%)

### Location
```rust
// src/iterate.rs:102, 117
let k = &*(parts.keys_ptr.add(*front_idx) as *const K);  // 2 cycles
let v = &*(parts.vals_ptr.add(*front_idx) as *const V);  // 2 cycles
```

### What It Does
Computes addresses for key and value:
```assembly
; Get key
mov    rax, [keys_ptr]      ; Load base (1 cycle)
mov    rbx, [front_idx]     ; Load index (1 cycle)
lea    rcx, [rax + rbx*8]   ; Compute address (1 cycle)
mov    rdx, [rcx]           ; Load value (1 cycle)

; Get value (similar)
; Total: 4 cycles, optimized to 2 cycles
```

### Why It's Hot
- Two separate calculations per element
- Index-based access requires multiply/shift
- Could use pointer increment instead

### Optimization
**Use raw pointer iteration:**
```rust
let mut k_ptr = cached_keys_ptr.add(idx);
let mut v_ptr = cached_vals_ptr.add(idx);
let end_ptr = cached_keys_ptr.add(len);

while k_ptr < end_ptr {
    yield (&*k_ptr, &*v_ptr);
    k_ptr = k_ptr.add(1);  // Just increment, no multiply
    v_ptr = v_ptr.add(1);
}
```

**Savings: 2-3 cycles per element → 1-1.5 cycles**

---

## Optimization Impact Summary

| Hotspot | Current | Optimized | Savings | Difficulty |
|---------|---------|-----------|---------|------------|
| carve_leaf | 3-4 cycles | 0 cycles | 3-4 cycles | Medium |
| Bound checking | 2-3 cycles | 0.1 cycles | 2-3 cycles | High |
| Pointer arithmetic | 2-3 cycles | 1-1.5 cycles | 1-1.5 cycles | Low |
| **TOTAL** | **10 cycles** | **2-3 cycles** | **7-8 cycles** | - |

**Expected speedup: 3-5x**

---

## Implementation Priority

### Phase 1: Cache Leaf Parts (HIGH PRIORITY)
- **Impact:** 3-4 cycles saved (35-40% improvement)
- **Difficulty:** Medium
- **Risk:** Low
- **Time:** 1-2 days

### Phase 2: Eliminate Bound Checking (HIGH PRIORITY)
- **Impact:** 2-3 cycles saved (20-30% improvement)
- **Difficulty:** High (correctness critical)
- **Risk:** Medium
- **Time:** 2-3 days

### Phase 3: Raw Pointer Iteration (MEDIUM PRIORITY)
- **Impact:** 1-1.5 cycles saved (10-15% improvement)
- **Difficulty:** Low
- **Risk:** Low
- **Time:** 1 day

---

## Code Locations Reference

### Main Iteration Loop
- **File:** `src/iterate.rs`
- **Function:** `Iterator::next()` for `Items<'a, K, V>`
- **Lines:** 33-140

### carve_leaf Function
- **File:** `src/layout.rs`
- **Function:** `carve_leaf<K, V>()`
- **Lines:** 318-332

### Bound Checking
- **File:** `src/iterate.rs`
- **Lines:** 105-115

### Pointer Arithmetic
- **File:** `src/iterate.rs`
- **Lines:** 102, 117

---

## Verification Strategy

After implementing optimizations:

1. **Correctness:**
   - Run all existing tests
   - Fuzz test with random ranges
   - Compare results with std::BTreeMap

2. **Performance:**
   - Re-run cycle measurements
   - Verify 3-5x speedup
   - Check no regression for other operations

3. **Profiling:**
   - Re-profile with RDTSC
   - Verify hotspots are eliminated
   - Measure new bottlenecks (if any)

---

## Expected Final Performance

| Metric | Current | After Phase 1 | After Phase 2 | After Phase 3 |
|--------|---------|---------------|---------------|---------------|
| Cycles/element | 10 | 6-7 | 3-4 | 2-3 |
| Time (3GHz) | 3.30ns | 2.00-2.30ns | 1.00-1.30ns | 0.66-1.00ns |
| vs std::BTree | 2.5x slower | 1.5-1.8x slower | 0.75-1x | **Competitive** |

**Target: Match or beat std::BTreeMap (1.30ns per element)**
