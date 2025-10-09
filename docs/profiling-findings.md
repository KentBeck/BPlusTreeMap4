# Profiling Findings - Insert Performance

## Date: 2025-10-09

## Summary

Profiling of sequential insert operations revealed that **78% of execution time** is spent in `memmove` operations, all originating from a single code path: `insert_into_leaf_slot` → `shift_right`.

## Profiling Setup

- **Tool**: macOS Instruments (Time Profiler)
- **Test**: `profile_insert` - 100,000 sequential inserts
- **Configuration**: All `#[inline(always)]` attributes temporarily removed to see call tree
- **Binary**: `profile_insert_noinline`

## Key Findings

### Call Tree Analysis

```
profile_insert::main (100%)
└── insert (100%)
    └── insert_rec (97%)
        └── insert_rec (recursive calls, 93% → 85% → 83%)
            └── insert_into_leaf_slot (80%)
                └── shift_right (79%)
                    └── core::intrinsics::copy (78%)
                        └── _platform_memmove (78% - 1,768 samples)
```

### Performance Breakdown

| Function | Samples | % | Notes |
|----------|---------|---|-------|
| `_platform_memmove` | 1,768 | 78% | Platform memmove implementation |
| `shift_right` | 1,797 | 79% | Calls `copy` twice (keys + values) |
| `insert_into_leaf_slot` | 1,814 | 80% | Calls `shift_right` |
| `insert_rec` (all levels) | 2,204 | 97% | Recursive traversal |

### Root Cause

The `shift_right` function performs **two separate `copy` operations**:

```rust
pub(crate) unsafe fn shift_right(
    &self,
    keys_ptr: *mut K,
    vals_ptr: *mut V,
    idx: usize,
    len: usize,
) {
    if idx < len {
        core::ptr::copy(keys_ptr.add(idx), keys_ptr.add(idx + 1), len - idx);
        core::ptr::copy(vals_ptr.add(idx), vals_ptr.add(idx + 1), len - idx);
    }
}
```

Each `core::ptr::copy` becomes a `memmove` call, resulting in:
- **2 memmove calls per insert** (one for keys, one for values)
- For 100,000 inserts, this is ~200,000 memmove operations
- Even when `idx == len` (append case), function call overhead exists

## Optimization Opportunities

### 1. Re-enable Inlining (High Priority)
- Restore `#[inline(always)]` on `shift_right` and `insert_into_leaf_slot`
- Eliminates function call overhead
- Allows compiler to optimize across function boundaries

### 2. Fast Path for Append (Medium Priority)
- Sequential inserts often append to the end of a leaf
- When `idx == len`, no shifting is needed
- Could add early return in `insert_into_leaf_slot`:
  ```rust
  if idx == cur_len {
      // Direct write, no shift needed
      self.write_kv_at(...);
      (*parts.hdr).len = (cur_len + 1) as u16;
      return;
  }
  ```

### 3. Combined Key-Value Shift (Low Priority, Complex)
- Current design uses separate arrays for keys and values
- Combining shifts would require layout changes
- Likely not worth the complexity

## Recommendations

1. **Immediate**: Restore all `#[inline(always)]` attributes
2. **Next**: Add fast path for append case (`idx == len`)
3. **Future**: Consider if the separate key/value array design is optimal

## Notes

- The profiling was done with inlining disabled to see the call tree
- Production builds should have inlining enabled
- The 78% memmove time is expected for array-based data structures
- The real question is whether we can reduce the number of shifts, not eliminate them

## Files Modified for Profiling (to be reverted)

- `src/common.rs` - removed `#[inline(always)]` from `shift_right`
- `src/insert.rs` - removed `#[inline(always)]` from multiple functions
- `src/bin/profile_insert_noinline.rs` - created for profiling

