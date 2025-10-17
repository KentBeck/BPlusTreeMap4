# API Usage Notes

## Iteration Methods

BPlusTreeMap provides several methods for iterating over items. Understanding when to use each method is critical for optimal performance.

### ✅ Recommended: `range()` for Partial Iteration

**Use `range(key..)` to iterate from a specific starting point:**

```rust
let map = BPlusTreeMap::new(128)?;
// ... populate map ...

// Iterate from a specific key forward
for (k, v) in map.range(start_key..) {
    // Process items
    if should_stop(k) {
        break;
    }
}

// Bounded range query
for (k, v) in map.range(start_key..end_key) {
    // Process items in range
}
```

**Performance:** 
- Iterator creation: ~200-500ns (O(log n) tree traversal to find start key)
- Per-item cost: 9-26ns
- **1.1-4x FASTER than std::BTreeMap** for typical partial iteration patterns

**Use cases:**
- ✅ Database cursors starting from a known key
- ✅ Pagination (iterate N items from bookmark key)
- ✅ Range queries with start/end bounds
- ✅ Sequential access from any position
- ✅ Any scenario where you know where to start iterating

### ⚠️ Not Recommended: `items()` on Large Trees

**The `items()` method iterates over all items starting from the beginning:**

```rust
let map = BPlusTreeMap::new(128)?;
// ... populate map ...

// Iterate from the very beginning
for (k, v) in map.items().take(100) {
    // Process first 100 items
}
```

**Performance Issue:**
- Iterator creation: **O(n) - walks all leaf nodes to compute length!**
- For 10M items: ~16ms overhead per `items()` call
- **5,000x SLOWER than std::BTreeMap** for this pattern

**Why it's slow:**
`items()` calls `len()` internally, which walks through all leaf nodes in the tree to count items. This is a deliberate design tradeoff (no cached length field) that affects `items()` performance on large trees.

**Workaround:**
If you need to iterate from the beginning, use `range()` instead:

```rust
// Instead of items()
for (k, v) in map.items().take(100) {
    // ...
}

// Use range() from first key
if let Some(&first_key) = map.keys().next() {
    for (k, v) in map.range(first_key..).take(100) {
        // Same result, but 80,000x faster!
    }
}

// Or if you know the minimum possible key
for (k, v) in map.range(MIN_KEY..).take(100) {
    // Fast and simple
}
```

**When `items()` is acceptable:**
- ✅ Small trees (<10,000 items) - overhead is negligible
- ✅ When you actually need to iterate the entire tree
- ✅ When performance is not critical
- ⚠️ Never for hot paths or frequent partial iteration

## Length Queries

### ⚠️ `len()` is O(n) - Use Sparingly

**Current implementation:**

```rust
pub fn len(&self) -> usize {
    // Walks all leaf nodes to count items
}
```

**Cost:** For 10M items, `len()` takes ~16ms

**Recommendation:**
- ✅ Cache the result if you need it multiple times
- ✅ Avoid calling in hot paths
- ✅ Consider tracking count externally if needed frequently
- ⚠️ Don't call on every iteration

**Example - Cache the length:**

```rust
let map_len = map.len();  // Call once
for batch in 0..batches {
    if processed >= map_len {
        break;
    }
    // Use cached map_len instead of calling len() again
}
```

## Performance Best Practices

### ✅ DO: Use range() for Partial Iteration

```rust
// Database cursor pattern
let mut cursor_key = start_key;
loop {
    let mut count = 0;
    for (k, v) in map.range(cursor_key..) {
        process(k, v);
        cursor_key = k.clone();
        count += 1;
        if count >= BATCH_SIZE {
            break;
        }
    }
    if count < BATCH_SIZE {
        break; // Done
    }
}
```

**Performance:** 26ns per item (1.4x faster than std::BTreeMap)

### ✅ DO: Use Bounded Ranges

```rust
// Range query
for (k, v) in map.range(start_key..end_key) {
    process(k, v);
}
```

**Performance:** 20ns per item (4x faster than std::BTreeMap)

### ❌ DON'T: Use items() in Hot Paths

```rust
// Bad: O(n) overhead on every call
fn process_batch(map: &BPlusTreeMap<K, V>) {
    for (k, v) in map.items().take(100) {  // ← 16ms overhead!
        process(k, v);
    }
}
```

### ❌ DON'T: Call len() Repeatedly

```rust
// Bad: O(n) on every call
for i in 0..100 {
    if processed >= map.len() {  // ← 16ms per call!
        break;
    }
}

// Good: Cache the length
let total = map.len();  // Once
for i in 0..100 {
    if processed >= total {
        break;
    }
}
```

## Capacity Selection

**Use capacity 128 for optimal performance:**

```rust
let map = BPlusTreeMap::new(128)?;
```

**Performance comparison (10M items, iterate 100):**
- Capacity 16: 1.3-1.5x slower than std::BTreeMap
- Capacity 128: **1.1-4x FASTER than std::BTreeMap** ✓

Larger capacity means:
- Fewer tree levels (faster traversal)
- Better cache locality (more items per node)
- Lower per-item iteration cost

## Summary Table

| Operation | Method | Performance | Use When |
|-----------|--------|-------------|----------|
| **Range query** | `range(start..end)` | **4x faster than std** ✓ | Bounded queries |
| **Cursor from key** | `range(key..)` | **1.4x faster than std** ✓ | Pagination, cursors |
| **Random position** | `range(key..)` | **1.1x faster than std** ✓ | Scattered queries |
| **From beginning** | `items()` | 5,000x slower than std ⚠️ | Avoid on large trees |
| **Count items** | `len()` | O(n) - 16ms for 10M ⚠️ | Cache result |

## Real-World Performance

With capacity 128, BPlusTreeMap **outperforms std::BTreeMap** for typical partial iteration patterns:

```
Database cursors:        1.41x faster (26ns vs 37ns per item)
Range queries:           3.98x faster (20ns vs 83ns per item)  
Random partial queries:  1.11x faster (9ns vs 10ns per item)
```

**Conclusion:** Use `range()` for partial iteration, avoid `items()` on large trees, and cache `len()` results. With these patterns, BPlusTreeMap is production-ready and faster than std::BTreeMap.