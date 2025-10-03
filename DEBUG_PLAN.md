# Debug Plan: Double Free in Branch Rebalancing

## Problem Summary
Double free error when dropping String values after branch node operations. The same String is being freed twice, indicating either:
1. Duplicate ownership (same value stored in two places)
2. Use-after-free (value freed but pointer still referenced)
3. Incorrect ownership transfer during node operations

## Phase 1: Reproduce and Isolate (DONE)
✅ Isolated failing test: `test_branch_borrow_from_underfull_sibling_attack`
✅ Created minimal reproduction case
✅ Confirmed double free in String drop during cleanup
✅ Stack trace shows: drop happens at end of main(), not during operations

## Phase 2: Identify the Exact Operation Causing the Bug ✅ COMPLETED

### Findings:
- Double free occurs during deletion of key=48 in the test sequence
- Crash happens DURING the remove operation, not during final cleanup
- The issue is in branch/leaf rebalancing operations
- Instrumentation shows all values have unique heap addresses
- The double free is triggered by tree rebalancing, specifically during merge operations

### Key Observations:
1. Test inserts 16 keys, all get unique heap addresses
2. Deletes keys 18, 28, 38 successfully
3. Deletion of key 48 triggers the double free
4. The crash occurs before we can check invariants after the deletion

## Phase 2: Identify the Exact Operation Causing the Bug

### 2.1 Add Instrumentation
Add debug logging to track value ownership:
- Log every value insertion with memory address
- Log every value removal with memory address
- Log every value move during node operations (split, merge, borrow)
- Track which node owns which value at each step

### 2.2 Narrow Down the Trigger
Run the minimal test with instrumentation to find:
- Which specific `remove()` call causes the duplicate ownership
- What tree structure exists at that moment
- Which branch operation (borrow/merge) is involved

### 2.3 Expected Findings
The bug likely occurs in one of these operations:
- `borrow_from_left_branch()` / `borrow_from_right_branch()`
- `merge_with_left_branch()` / `merge_with_right_branch()`
- Branch node splitting during rebalancing
- Parent key updates during branch operations

## Phase 3: Analyze the Code for Ownership Violations

### 3.1 Review Branch Borrowing Logic
Check `src/branch_ops.rs` or equivalent for:
- Does borrowing copy values instead of moving them?
- Are parent keys properly transferred during borrow?
- Is the separator key in parent updated correctly?
- Are child pointers properly transferred?

### 3.2 Review Branch Merging Logic
Check for:
- Are all values from merged node properly moved (not copied)?
- Is the merged node properly deallocated?
- Are parent keys removed correctly?
- Are child pointers consolidated correctly?

### 3.3 Review Parent Key Management
Critical: In B+ trees, parent nodes contain separator keys that may be:
- Copies of child keys (if using copy semantics)
- References to child keys (dangerous if child is freed)
- Owned separately (correct approach)

Check if parent keys are:
- Cloned when inserted into parent
- Properly updated when child keys change
- Properly freed when parent node is deallocated

### 3.4 Common Bug Patterns to Look For

**Pattern 1: Shallow Copy Instead of Move**
```rust
// WRONG: Creates duplicate ownership
let value = node.values[i].clone(); // or copy
other_node.values[j] = value;
// Both nodes now own the value!

// CORRECT: Transfer ownership
let value = node.values.remove(i);
other_node.values.insert(j, value);
```

**Pattern 2: Parent Key Not Cloned**
```rust
// WRONG: Parent references child's key
let separator = &child.keys[0];
parent.keys.push(separator); // Stores reference, not owned copy

// CORRECT: Parent owns its own copy
let separator = child.keys[0].clone();
parent.keys.push(separator);
```

**Pattern 3: Forgetting to Remove After Move**
```rust
// WRONG: Value moved but not removed from source
let value = node.values[i].clone(); // Should be take/remove
target.values.push(value);
// node.values[i] still exists!

// CORRECT: Remove after taking
let value = node.values.remove(i);
target.values.push(value);
```

**Pattern 4: Double Free During Merge**
```rust
// WRONG: Merge copies values, then both nodes get dropped
fn merge(left: &mut Node, right: &Node) {
    for val in &right.values {
        left.values.push(val.clone()); // Clone creates duplicate
    }
    // right gets dropped later, freeing originals
    // left gets dropped, freeing clones
    // But if clone is shallow, DOUBLE FREE!
}

// CORRECT: Move values, prevent right from being dropped
fn merge(left: &mut Node, right: Node) {
    left.values.extend(right.values); // Moves ownership
    std::mem::forget(right); // Prevent drop
}
```

## Phase 4: Locate the Bug in Source Code

### 4.1 Search Strategy
1. Find all branch node operation functions
2. For each function, trace value ownership:
   - Where do values come from?
   - Where do they go?
   - Are they moved or copied?
   - Is the source cleared after move?

### 4.2 Key Files to Examine
Based on typical B+ tree structure:
- `src/lib.rs` - Main tree operations
- `src/delete.rs` or similar - Deletion and rebalancing
- `src/branch.rs` or similar - Branch node operations
- Look for functions with names like:
  - `borrow_from_*`
  - `merge_*`
  - `rebalance_*`
  - `update_parent_key`

### 4.3 Specific Code Patterns to Search For
```bash
# Find all places where values are cloned
grep -n "\.clone()" src/*.rs

# Find all places where values are moved from vectors
grep -n "\.remove\|\.swap_remove\|\.pop\|\.drain" src/*.rs

# Find branch merge/borrow operations
grep -n "merge.*branch\|borrow.*branch" src/*.rs

# Find parent key updates
grep -n "parent.*key\|separator" src/*.rs
```

## Phase 5: Design the Fix

### 5.1 Ownership Principles for B+ Tree
Establish clear ownership rules:

**Rule 1: Values are owned by leaf nodes only**
- Branch nodes contain keys (cloned from leaves)
- Branch nodes contain child pointers (NodeId/references)
- Branch nodes NEVER own the actual values

**Rule 2: Keys in branch nodes are independent copies**
- When promoting a key to parent, clone it
- Parent key lifetime is independent of child key
- Updating child keys doesn't affect parent keys

**Rule 3: Node operations must transfer ownership cleanly**
- Borrow: Move keys/values from source to destination, remove from source
- Merge: Move all keys/values from merged node, deallocate merged node
- Split: Move keys/values to new node, clear from old node

**Rule 4: Use Rust's type system to enforce ownership**
- Use `Vec::remove()` or `Vec::drain()` to take ownership
- Use `std::mem::take()` to replace with default
- Use `std::mem::forget()` if preventing drop is needed
- Avoid `clone()` unless explicitly needed for independent copy

### 5.2 Refactoring Strategy

**Step 1: Add Ownership Assertions**
Add debug assertions to verify ownership invariants:
```rust
#[cfg(debug_assertions)]
fn verify_no_duplicate_values(&self) {
    // Check that no value appears in multiple nodes
    // Use HashSet to track seen value addresses
}
```

**Step 2: Fix Branch Borrow Operations**
Ensure borrowing properly transfers ownership:
```rust
fn borrow_from_sibling(&mut self, sibling: &mut BranchNode, parent: &mut BranchNode) {
    // 1. Remove key from sibling (transfers ownership)
    let key = sibling.keys.remove(index);
    
    // 2. Update parent separator (clone for parent's independent copy)
    parent.keys[separator_index] = sibling.keys[new_index].clone();
    
    // 3. Insert key into self (now owns it)
    self.keys.insert(position, key);
    
    // 4. Transfer child pointer
    let child = sibling.children.remove(index);
    self.children.insert(position, child);
}
```

**Step 3: Fix Branch Merge Operations**
Ensure merging properly consolidates ownership:
```rust
fn merge_with_sibling(&mut self, sibling: BranchNode, separator: K) {
    // 1. Take separator from parent (parent no longer owns it)
    self.keys.push(separator);
    
    // 2. Move all keys from sibling (transfers ownership)
    self.keys.extend(sibling.keys);
    
    // 3. Move all children from sibling
    self.children.extend(sibling.children);
    
    // 4. Prevent sibling from being dropped (already moved everything)
    std::mem::forget(sibling);
}
```

**Step 4: Fix Parent Key Updates**
Ensure parent keys are independent:
```rust
fn update_parent_separator(&mut self, parent: &mut BranchNode, old_key: &K, new_key: &K) {
    if let Some(pos) = parent.keys.iter().position(|k| k == old_key) {
        // Clone the new key for parent's independent ownership
        parent.keys[pos] = new_key.clone();
    }
}
```

### 5.3 Testing Strategy

**Test 1: Ownership Tracking**
Add a test that tracks value addresses:
```rust
#[test]
fn test_value_ownership_during_operations() {
    let mut tree = BPlusTreeMap::new(4).unwrap();
    
    // Insert values and track their addresses
    let mut value_addresses = HashMap::new();
    for i in 0..20 {
        let value = format!("v{}", i);
        let addr = &value as *const String as usize;
        value_addresses.insert(i, addr);
        tree.insert(i, value);
    }
    
    // Perform operations that trigger rebalancing
    for i in 0..10 {
        tree.remove(&i);
    }
    
    // Verify no duplicate addresses in tree
    // (This would require exposing internal structure for testing)
}
```

**Test 2: Stress Test with Drop Tracking**
```rust
struct DropTracker {
    id: usize,
    dropped: Arc<AtomicBool>,
}

impl Drop for DropTracker {
    fn drop(&mut self) {
        if self.dropped.swap(true, Ordering::SeqCst) {
            panic!("Double drop detected for id {}", self.id);
        }
    }
}

#[test]
fn test_no_double_drops() {
    let mut tree = BPlusTreeMap::new(4).unwrap();
    let mut trackers = Vec::new();
    
    for i in 0..20 {
        let dropped = Arc::new(AtomicBool::new(false));
        let tracker = DropTracker { id: i, dropped: dropped.clone() };
        trackers.push(dropped);
        tree.insert(i, tracker);
    }
    
    // Trigger rebalancing
    for i in 0..10 {
        tree.remove(&i);
    }
    
    drop(tree);
    
    // Verify each value was dropped exactly once
    for (i, dropped) in trackers.iter().enumerate() {
        assert!(dropped.load(Ordering::SeqCst), "Value {} was never dropped", i);
    }
}
```

## Phase 6: Implementation Plan

### 6.1 Immediate Fix (Tactical)
1. Find the specific function causing the double free
2. Add proper ownership transfer (use `remove()` instead of indexing)
3. Add `std::mem::forget()` if needed to prevent double drop
4. Test that the specific failing test now passes

### 6.2 Comprehensive Fix (Strategic)
1. Audit all branch operations for ownership correctness
2. Establish and document ownership invariants
3. Add debug assertions to verify invariants
4. Refactor code to make ownership explicit
5. Add comprehensive tests for all branch operations
6. Run full test suite under valgrind/miri

### 6.3 Design Improvements
1. Consider using `Box<T>` or `Rc<T>` for values to make ownership explicit
2. Consider separating "key" type from "value" type more clearly
3. Add lifetime annotations if using references
4. Use Rust's type system to prevent ownership bugs at compile time

## Phase 7: Verification

### 7.1 Verification Checklist
- [ ] Minimal reproduction case passes
- [ ] All branch rebalancing tests pass
- [ ] No memory leaks (run under valgrind)
- [ ] No undefined behavior (run under miri)
- [ ] All existing tests still pass
- [ ] New tests added for ownership correctness

### 7.2 Tools to Use
```bash
# Check for memory leaks
cargo test --test adversarial_branch_rebalancing -- --test-threads=1 2>&1 | valgrind

# Check for undefined behavior (if miri supports the code)
cargo +nightly miri test

# Run with address sanitizer
RUSTFLAGS="-Z sanitizer=address" cargo +nightly test
```

## Investigation Status

### Code Reviewed:
- ✅ `merge_branch_with_left` / `merge_branch_with_right` - Uses `ptr::read` to move separator key
- ✅ `collapse_branch_entry` - Shifts keys without dropping (correct, since key already moved)
- ✅ `remove_branch_entry` - Properly drops key before shifting (used for leaf merges)
- ✅ `merge_leaf_into` - Uses `read_kv_at` which moves values, sets source len=0
- ✅ `free_branch_node` - Drops keys based on length
- ✅ `free_leaf_node` - Drops keys/values based on length

### Potential Issues Found:
1. **Two different functions for removing branch entries:**
   - `remove_branch_entry` - drops the key explicitly
   - `collapse_branch_entry` - does NOT drop the key (assumes already moved)
   - Both use `ptr::copy` to shift remaining keys
   - After shift, last position contains duplicate key
   - Length is decremented so duplicate is outside valid range
   - Should be safe, but needs verification

2. **Possible issue with `ptr::copy` overlap:**
   - When shifting keys left, source and destination overlap
   - `ptr::copy` should handle this, but maybe there's an edge case?

3. **Possible issue with parent key updates:**
   - When merging, parent separator key is moved to child
   - Parent keys are then shifted
   - Could there be a case where a key is referenced twice?

### Next Steps Needed:
1. Add detailed logging to track every `ptr::read`, `ptr::write`, `ptr::copy`, and `drop`
2. Use address sanitizer or valgrind to get exact location of double free
3. Check if issue is specific to String type or affects all types
4. Verify that `ptr::copy` with overlapping regions works correctly
5. Check if there's an issue with how parent keys are updated during merge

## Expected Root Cause

Based on the symptoms and common B+ tree bugs, the most likely root cause is:

**Hypothesis: Branch borrowing copies child pointer but doesn't remove it from source**

When a branch node borrows from a sibling:
1. It copies a child pointer from sibling to self
2. It updates the parent separator key
3. BUT it forgets to remove the child pointer from the sibling
4. Now both nodes reference the same child
5. When both nodes are eventually dropped, the child is freed twice
6. Since the child contains values, those values are freed twice → DOUBLE FREE

**Alternative Hypothesis: Parent key is not cloned**

When updating parent separator keys:
1. Parent stores a reference to child's key (not a clone)
2. Child node is freed during merge/rebalance
3. Parent still has dangling reference
4. When parent is dropped, it tries to free the already-freed key → DOUBLE FREE

## Next Steps

1. Add instrumentation to track value ownership
2. Run minimal test with logging to see exact operation sequence
3. Examine the specific branch operation that causes the bug
4. Apply the fix based on the identified pattern
5. Verify with comprehensive tests
