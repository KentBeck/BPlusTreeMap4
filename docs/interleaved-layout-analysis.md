# Interleaved Layout Analysis: K,V,K,V vs K,K,K...,V,V,V...

## Date: 2025-10-09

## Question
What would be the impact of storing key-value pairs in an interleaved layout (K,V,K,V,...) instead of the current separate array layout (K,K,K..., V,V,V...)?

## Current Layout (Separate Arrays)

```
Leaf Node:
[Header][Next*][Prev*][K₀][K₁][K₂]...[Kₙ][V₀][V₁][V₂]...[Vₙ]
                       └─ keys array ─┘ └─ values array ─┘

Branch Node:
[Header][C₀*][C₁*]...[Cₙ*][K₀][K₁]...[Kₙ₋₁]
        └─ children ptrs ─┘ └─ keys array ─┘
```

## Proposed Layout (Interleaved)

```
Leaf Node:
[Header][Next*][Prev*][(K₀,V₀)][(K₁,V₁)][(K₂,V₂)]...[(Kₙ,Vₙ)]
                       └────── interleaved pairs ──────────┘
```

---

## Impact Analysis by Operation

### 1. **INSERT** 

#### Current (Separate Arrays)
```rust
// shift_right does TWO memmove operations
core::ptr::copy(keys_ptr.add(idx), keys_ptr.add(idx + 1), len - idx);  // memmove #1
core::ptr::copy(vals_ptr.add(idx), vals_ptr.add(idx + 1), len - idx);  // memmove #2
```
- **Cost**: 2 × memmove calls
- **Bytes moved**: `(len - idx) × (sizeof(K) + sizeof(V))`
- **Cache**: Two separate memory regions accessed

#### Interleaved
```rust
// Single memmove of (K,V) pairs
core::ptr::copy(pairs_ptr.add(idx), pairs_ptr.add(idx + 1), len - idx);  // ONE memmove
```
- **Cost**: 1 × memmove call
- **Bytes moved**: `(len - idx) × sizeof((K,V))` — **SAME total bytes**
- **Cache**: Single contiguous region

**VERDICT**: ✅ **~2× FASTER** (eliminates one memmove call, better cache locality)

---

### 2. **LOOKUP** (get/contains_key)

#### Current (Separate Arrays)
```rust
// Binary search on keys only
let keys = slice::from_raw_parts(parts.keys_ptr, len);
let idx = keys.binary_search(key)?;
// Then access value
let value = &*parts.vals_ptr.add(idx);
```
- **Cache**: Keys are contiguous → excellent cache locality during search
- **Access pattern**: Sequential key reads, then one value read

#### Interleaved
```rust
// Binary search on interleaved pairs
let pairs = slice::from_raw_parts(parts.pairs_ptr, len);
let idx = pairs.binary_search_by_key(key, |(k, _)| k)?;
// Value is already in the pair
let value = &pairs[idx].1;
```
- **Cache**: Each comparison loads both K and V (even though V is unused)
- **Access pattern**: Loads extra data during search

**VERDICT**: ❌ **SLOWER** (wastes cache bandwidth loading unused values during binary search)

**Key insight**: Binary search only needs keys, but interleaved layout forces loading values too.

---

### 3. **ITERATION** (items/range)

#### Current (Separate Arrays)
```rust
for i in 0..len {
    let k = &*keys_ptr.add(i);
    let v = &*vals_ptr.add(i);
    yield (k, v);
}
```
- **Cache**: Two separate sequential scans (keys, then values)
- **Prefetcher**: Can predict both streams independently

#### Interleaved
```rust
for i in 0..len {
    let pair = &*pairs_ptr.add(i);
    yield (&pair.0, &pair.1);
}
```
- **Cache**: Single sequential scan
- **Prefetcher**: Single stream to predict

**VERDICT**: ≈ **NEUTRAL to SLIGHTLY FASTER** (single stream is simpler for prefetcher)

---

### 4. **SPLIT** (leaf/branch overflow)

#### Current (Separate Arrays)
```rust
// Split at midpoint: copy right half to new node
let mid = len / 2;
core::ptr::copy_nonoverlapping(
    keys_ptr.add(mid), right_keys_ptr, len - mid);  // Copy keys
core::ptr::copy_nonoverlapping(
    vals_ptr.add(mid), right_vals_ptr, len - mid);  // Copy values
```
- **Cost**: 2 × copy operations
- **Bytes copied**: `(len - mid) × (sizeof(K) + sizeof(V))`

#### Interleaved
```rust
// Single copy of pairs
core::ptr::copy_nonoverlapping(
    pairs_ptr.add(mid), right_pairs_ptr, len - mid);  // ONE copy
```
- **Cost**: 1 × copy operation
- **Bytes copied**: `(len - mid) × sizeof((K,V))` — **SAME total bytes**

**VERDICT**: ✅ **~2× FASTER** (one copy instead of two)

---

### 5. **MERGE** (leaf/branch underflow)

#### Current (Separate Arrays)
```rust
// Merge right node into left
for i in 0..right_len {
    core::ptr::write(left_keys.add(left_len + i), 
                     core::ptr::read(right_keys.add(i)));
    core::ptr::write(left_vals.add(left_len + i), 
                     core::ptr::read(right_vals.add(i)));
}
```
- **Cost**: 2 × loops, 2 × memory regions

#### Interleaved
```rust
// Single copy
core::ptr::copy_nonoverlapping(
    right_pairs_ptr, left_pairs_ptr.add(left_len), right_len);
```
- **Cost**: 1 × copy operation

**VERDICT**: ✅ **~2× FASTER**

---

### 6. **DELETE** (remove from leaf)

#### Current (Separate Arrays)
```rust
// Shift left to fill gap
core::ptr::copy(keys_ptr.add(idx + 1), keys_ptr.add(idx), len - idx - 1);
core::ptr::copy(vals_ptr.add(idx + 1), vals_ptr.add(idx), len - idx - 1);
```
- **Cost**: 2 × memmove

#### Interleaved
```rust
// Single shift
core::ptr::copy(pairs_ptr.add(idx + 1), pairs_ptr.add(idx), len - idx - 1);
```
- **Cost**: 1 × memmove

**VERDICT**: ✅ **~2× FASTER**

---

### 7. **KEYS-ONLY ITERATION** (keys())

#### Current (Separate Arrays)
```rust
for i in 0..len {
    yield &*keys_ptr.add(i);
}
```
- **Cache**: Only loads keys array (optimal)

#### Interleaved
```rust
for i in 0..len {
    yield &pairs_ptr.add(i).0;
}
```
- **Cache**: Loads entire (K,V) pairs even though V is unused

**VERDICT**: ❌ **SLOWER** (wastes bandwidth loading unused values)

---

## Summary Table

| Operation | Current (Separate) | Interleaved | Winner | Speedup |
|-----------|-------------------|-------------|--------|---------|
| **Insert** | 2 memmoves | 1 memmove | ✅ Interleaved | ~2× |
| **Lookup** | Keys-only search | Search loads K+V | ❌ Separate | ~1.5-2× |
| **Iteration (items)** | 2 streams | 1 stream | ≈ Neutral | ~1.0× |
| **Split** | 2 copies | 1 copy | ✅ Interleaved | ~2× |
| **Merge** | 2 copies | 1 copy | ✅ Interleaved | ~2× |
| **Delete** | 2 memmoves | 1 memmove | ✅ Interleaved | ~2× |
| **Keys-only iteration** | Keys array | Loads K+V | ❌ Separate | ~2× |

---

## Overall Assessment

### Interleaved Layout WINS for:
- ✅ **Write-heavy workloads** (insert, delete, split, merge)
- ✅ **Full iteration** (items)
- ✅ **Simpler code** (fewer operations)

### Separate Layout WINS for:
- ✅ **Read-heavy workloads** (lookup, contains_key)
- ✅ **Keys-only operations** (keys(), binary search)
- ✅ **Cache efficiency during search** (critical for B+ trees)

---

## Critical Consideration: **B+ Trees are Search-Optimized**

B+ trees are designed for **fast lookups**. The most common operation is:
1. **Search** (traverse tree via binary search on keys)
2. **Lookup** (find value for key)

**Separate arrays optimize the hot path**: binary search only touches keys, maximizing cache efficiency.

**Interleaved layout penalizes the hot path**: every comparison loads unused values.

---

## Recommendation

**KEEP SEPARATE ARRAYS** for this B+ tree implementation because:

1. **Lookups are more frequent than inserts** in most use cases
2. **Binary search is the critical path** (happens on every operation)
3. **Cache efficiency during search** is paramount
4. **The 2× insert speedup is less valuable** than maintaining fast lookups

### When Interleaved Would Be Better:
- **Write-heavy workloads** (logging, time-series data)
- **Small value types** (where loading V doesn't hurt much)
- **No keys-only operations**
- **Sequential access patterns** (iteration-heavy)

### Hybrid Approach (Future):
- Use **separate arrays in branch nodes** (search-optimized)
- Use **interleaved pairs in leaf nodes** (write-optimized)
- This gives fast search + fast leaf modifications

---

## Alignment Considerations

### Current Layout
```
Keys:   aligned to align_of::<K>()
Values: aligned to align_of::<V>()
```
- Each array has optimal alignment

### Interleaved Layout
```
Pairs: aligned to align_of::<(K,V)>()
```
- May waste space due to padding between K and V in each pair
- Example: `(u32, u64)` → 4 bytes padding per pair

**Memory overhead**: Interleaved can use **more memory** due to per-pair padding.

---

## Conclusion

The current **separate array layout is the right choice** for a general-purpose B+ tree because it optimizes the most critical operation: **key lookup via binary search**.

The profiling showed that 78% of insert time is in memmove, but **changing to interleaved layout would make lookups slower**, which is unacceptable for a search tree.

**Better optimizations**:
1. Re-enable inlining (done)
2. Add fast path for append case (idx == len)
3. Use SIMD for bulk shifts (advanced)
4. Optimize node capacity to reduce splits

