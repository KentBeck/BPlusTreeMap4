# Double-Free Bug Analysis

## Summary

The double-free crash is a **symptom** of a deeper bug: **branch node overflow during merge operations**. The tree becomes corrupted after the first remove operation, and subsequent operations work with corrupted state, eventually triggering a double-free when using types with drop tracking.

## The Bug

### Primary Issue: Branch Overflow

**Location**: `src/delete.rs`, `merge_branch_with_left()` and `merge_branch_with_right()`

**Problem**: When merging two branch nodes, the code does not check if the combined size exceeds the node capacity.

```rust
// Line 385 in merge_branch_with_left
(*left_parts.hdr).len = (left_len + 1 + child_len) as u16;
// No capacity check!
```

### How It Happens

With capacity = 5:
- Minimum keys per node = (5+1)/2 = 3
- When a node becomes underfull (< 3 keys), rebalancing is triggered

**Scenario**:
1. Child node has 2 keys (underfull)
2. Left sibling has 3 keys (at minimum, cannot lend)
3. Right sibling has 3 keys (at minimum, cannot lend)
4. Code decides to merge child with left
5. **Merged size = 3 + 1 (separator) + 2 = 6 keys**
6. **Capacity is only 5 → OVERFLOW**

### Reproduction

```rust
let mut tree: BPlusTreeMap<i32, String> = BPlusTreeMap::new(5).unwrap();

// Insert 50 items
for i in 0..50 {
    tree.insert(i, format!("value_{}", i));
}

// After insert: 16 leaves, multi-level tree, invariants OK

tree.remove(&10);
// After remove: Branch has 6 keys but capacity is 5 ← CORRUPTED

tree.check_invariants_detailed();
// Returns: Err("Branch has 6 keys but capacity is 5")
```

## Why It Causes Double-Free

The overflow creates corrupted tree state:

1. **Branch node has 6 keys** (positions 0-5) when capacity is 5 (positions 0-4)
2. The 6th key at position 5 is **beyond allocated memory** or in padding
3. Subsequent operations (remove 11, 12, 13) work with this corrupted state
4. When manipulating keys, the code may:
   - Read the invalid 6th key
   - Drop it
   - Later try to drop it again from a different code path
   - **Result: Double-free**

With regular types like `i32`, this manifests as:
- Memory corruption
- Invalid data
- Invariant violations
- But no immediate crash

With `DropCounter` (which tracks drops), this manifests as:
- **Immediate crash** when the same object is dropped twice
- The drop counter underflows (goes negative)
- Allocator detects "double free or corruption"

## The Flawed Assumption

The rebalancing code assumes:

> "If neither sibling can lend a key, they must both be at minimum size,  
> so merging will fit within capacity."

This is **FALSE** because:
- Child can be at (min - 1) keys
- Sibling can be at exactly min keys
- Merged = (min - 1) + 1 + min = 2*min
- But capacity might be < 2*min

**Example with capacity 5**:
- min = 3
- Child has 2 keys (min - 1)
- Sibling has 3 keys (min)
- Merged = 2 + 1 + 3 = 6
- Capacity = 5
- **6 > 5 → OVERFLOW**

## Why Remove 13 Crashes

Remove 13 doesn't cause the corruption - it just triggers the crash:

1. **Remove 10**: Creates the overflow (6 keys in branch)
2. **Remove 11**: Works with corrupted state, no crash yet
3. **Remove 12**: Works with corrupted state, no crash yet
4. **Remove 13**: Triggers rebalancing that tries to manipulate the overflowed branch
   - Reads a key from the corrupted region
   - Tries to drop it
   - Later tries to drop it again
   - **CRASH: double free detected**

## Impact

This is a **critical correctness bug**:

1. **Data Structure Invariant Violated**: Nodes exceed capacity
2. **Memory Safety Violated**: Double-free, potential use-after-free
3. **Silent Corruption**: With non-drop types, corruption is silent
4. **Unpredictable Behavior**: Depends on memory layout, allocation patterns

## The Fix (Not Implemented)

Before merging, check if combined size fits:

```rust
unsafe fn merge_branch_with_left(&mut self, branch: NonNull<u8>, child_idx: usize) {
    // ... existing code to get left_len and child_len ...
    
    // CHECK: Will merge fit?
    let merged_len = left_len + 1 + child_len;
    if merged_len > self.branch_layout.cap as usize {
        // Cannot merge - would overflow
        // Need different strategy (split parent, redistribute, etc.)
        panic!("Cannot merge: would exceed capacity");
    }
    
    // ... proceed with merge ...
}
```

But the real fix is more complex:
- Need to handle the case where merge doesn't fit
- May need to propagate changes up the tree
- May need to split the parent node
- Requires careful redesign of the rebalancing algorithm

## Conclusion

The double-free is not a simple memory management bug. It's a **fundamental flaw in the rebalancing algorithm** that violates the B+ tree invariant that nodes must not exceed capacity. The algorithm needs to be redesigned to handle cases where merging would cause overflow.

This is why the data structure is **not production-ready** - it has a critical correctness bug that can corrupt data and cause crashes.
