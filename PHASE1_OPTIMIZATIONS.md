# Phase 1 Delete Optimizations - Implementation Guide

## Overview

Phase 1 focuses on **quick wins** with minimal risk:
- Expected improvement: **10-15%**
- Implementation time: **1-2 days**
- Risk level: **Low**

---

## Optimization 1: Optimize Root Collapse Check

### Current Issue
```rust
// src/delete.rs:5-12
pub fn remove(&mut self, key: &K) -> Option<V> {
    let root = self.root?;
    let result = unsafe { self.remove_rec(root, key) };
    if result.is_some() {
        unsafe { self.check_root_collapse() };  // ❌ Called EVERY delete
    }
    result
}
```

**Problem:** `check_root_collapse()` is called after every successful delete, even when the root is a leaf or has many children.

### Proposed Fix

```rust
// src/delete.rs:5-17
pub fn remove(&mut self, key: &K) -> Option<V> {
    let root = self.root?;
    let result = unsafe { self.remove_rec(root, key) };
    if result.is_some() {
        // Only check root collapse if root is a branch with few children
        unsafe {
            if let Some(root) = self.root {
                let hdr = &*(root.as_ptr() as *const NodeHdr);
                if hdr.tag == NodeTag::Branch && (*hdr).len <= 2 {
                    self.check_root_collapse();
                }
            }
        }
    }
    result
}
```

**Benefits:**
- Avoids unnecessary checks when root is a leaf
- Avoids checks when root branch has many children
- Expected gain: **5-10%**

---

## Optimization 2: Add Inline Annotations

### Current Issue

Some hot-path functions may not be inlined by the compiler, causing function call overhead.

### Proposed Fixes

#### 2.1 Ensure `child_for_key` is always inlined

```rust
// src/common.rs:100-115
#[inline(always)]  // ✅ Change from #[inline] to #[inline(always)]
pub(crate) unsafe fn child_for_key(
    &self,
    branch: NonNull<u8>,
    key: &K,
) -> Option<(NonNull<u8>, usize)> {
    let parts = layout::carve_branch::<K>(branch, &self.branch_layout);
    let len = (*parts.hdr).len as usize;
    let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
    let child_idx = match self.binary_search_keys(keys, key) {
        Ok(i) => i + 1,
        Err(i) => i,
    };
    let child_ptr = *(parts.children_ptr.add(child_idx) as *const *mut u8);
    NonNull::new(child_ptr).map(|child| (child, child_idx))
}
```

#### 2.2 Ensure `leaf_for_key` is always inlined

```rust
// src/common.rs:117-135
#[inline(always)]  // ✅ Change from #[inline] to #[inline(always)]
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

#### 2.3 Ensure helper functions are inlined

```rust
// src/common.rs:399-413
#[inline(always)]  // ✅ Add inline annotation
pub(crate) fn min_leaf_len(&self) -> usize {
    let cap = self.leaf_layout.cap as usize;
    cap / 2
}

#[inline(always)]  // ✅ Add inline annotation
pub(crate) fn min_branch_len(&self) -> usize {
    let cap = self.branch_layout.cap as usize;
    if cap <= 2 {
        1
    } else {
        cap / 2
    }
}
```

**Benefits:**
- Reduces function call overhead in hot paths
- Expected gain: **2-3%**

---

## Optimization 3: Batch Memory Operations

### Current Issue

Multiple small memory copy operations in sequence:

```rust
// src/delete.rs:694-704
if idx < len - 1 {
    core::ptr::copy(
        parts.keys_ptr.add(idx + 1) as *const K,
        parts.keys_ptr.add(idx) as *mut K,
        len - idx - 1,
    );
    core::ptr::copy(
        parts.vals_ptr.add(idx + 1) as *const V,
        parts.vals_ptr.add(idx) as *mut V,
        len - idx - 1,
    );
}
```

### Proposed Fix

Create a helper function for batched key-value shifts:

```rust
// src/common.rs - Add new helper function
#[inline(always)]
pub(crate) unsafe fn shift_left_kv(
    &self,
    keys_ptr: *mut K,
    vals_ptr: *mut V,
    start_idx: usize,
    count: usize,
) {
    if count > 0 {
        // Batch copy both keys and values
        core::ptr::copy(
            keys_ptr.add(start_idx + 1) as *const K,
            keys_ptr.add(start_idx) as *mut K,
            count,
        );
        core::ptr::copy(
            vals_ptr.add(start_idx + 1) as *const V,
            vals_ptr.add(start_idx) as *mut V,
            count,
        );
    }
}
```

Then use it in `leaf_remove`:

```rust
// src/delete.rs:683-713
unsafe fn leaf_remove(&mut self, leaf: NonNull<u8>, key: &K) -> Option<V> {
    let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
    let len = (*parts.hdr).len as usize;
    let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
    let idx = self.binary_search_keys(keys, key).ok()?;

    // Read the key and value (transferring ownership)
    let removed_key = core::ptr::read((parts.keys_ptr as *const K).add(idx));
    let value = core::ptr::read(parts.vals_ptr.add(idx) as *const V);

    // Shift remaining elements using batched operation
    if idx < len - 1 {
        self.shift_left_kv(
            parts.keys_ptr as *mut K,
            parts.vals_ptr as *mut V,
            idx,
            len - idx - 1,
        );
    }

    (*parts.hdr).len = (len - 1) as u16;

    // Drop the removed key (value is returned to caller)
    drop(removed_key);

    Some(value)
}
```

**Benefits:**
- Reduces function call overhead
- Better code organization
- Potential for compiler optimizations
- Expected gain: **3-5%**

---

## Implementation Checklist

### Step 1: Optimize Root Collapse Check
- [ ] Modify `remove()` function in `src/delete.rs`
- [ ] Add condition to check root tag and length
- [ ] Test with existing test suite
- [ ] Benchmark with `bench_delete`

### Step 2: Add Inline Annotations
- [ ] Change `child_for_key` to `#[inline(always)]`
- [ ] Change `leaf_for_key` to `#[inline(always)]`
- [ ] Add `#[inline(always)]` to `min_leaf_len`
- [ ] Add `#[inline(always)]` to `min_branch_len`
- [ ] Test with existing test suite
- [ ] Benchmark with `bench_delete`

### Step 3: Batch Memory Operations
- [ ] Add `shift_left_kv` helper to `src/common.rs`
- [ ] Update `leaf_remove` to use new helper
- [ ] Consider adding similar helpers for other operations
- [ ] Test with existing test suite
- [ ] Benchmark with `bench_delete`

### Step 4: Validate Improvements
- [ ] Run full test suite: `cargo test`
- [ ] Run benchmarks: `./target/release/bench_delete`
- [ ] Compare results with baseline
- [ ] Document actual improvements achieved

---

## Testing Commands

```bash
# Build with optimizations
cargo build --release --bin bench_delete

# Run benchmarks
./target/release/bench_delete

# Run test suite
cargo test

# Profile with callgrind (optional)
ulimit -n 1024
valgrind --tool=callgrind --callgrind-out-file=callgrind.phase1.out \
  ./target/release/profile_delete_small
callgrind_annotate --auto=yes callgrind.phase1.out | head -100
```

---

## Expected Results

### Before Phase 1
```
Testing with 1000000 items:
  BPlusTreeMap (cap=128):
    Delete: 0.243s (4110644 ops/sec)
  std::BTreeMap:
    Delete: 0.270s (3699736 ops/sec)
  Ratio (BPlusTree/std):
    Delete: 0.90x (faster)
```

### After Phase 1 (Target)
```
Testing with 1000000 items:
  BPlusTreeMap (cap=128):
    Delete: 0.210s (4761905 ops/sec)  ← 15% improvement
  std::BTreeMap:
    Delete: 0.270s (3699736 ops/sec)
  Ratio (BPlusTree/std):
    Delete: 0.78x (faster)  ← 22% faster than std
```

---

## Risk Assessment

| Optimization | Risk Level | Reason |
|--------------|-----------|---------|
| Root collapse check | **Low** | Only changes when check is performed, not the logic |
| Inline annotations | **Very Low** | Compiler hint, no logic change |
| Batch memory ops | **Low** | Refactoring existing operations, same semantics |

**Overall Risk:** ✅ **Low** - All changes are conservative and maintain existing semantics.

---

## Next Steps After Phase 1

1. Measure actual improvements achieved
2. If gains are ≥10%, proceed to Phase 2
3. If gains are <10%, investigate further with profiling
4. Document lessons learned for Phase 2 planning

---

## Notes

- All changes should maintain existing test suite compatibility
- No changes to public API
- Focus on hot paths identified by profiling
- Keep changes small and testable
- Benchmark after each optimization to isolate impact
