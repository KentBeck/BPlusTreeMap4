# Leaf Split Refactoring Plan

## Current Problem

`leaf_insert_or_split` is doing too much and can't be tested or optimized in isolation:

- **Mixed responsibilities**: Core algorithm + memory management + tree state updates
- **Too many dependencies**: Needs 8+ tree methods and fields
- **Hard to test**: Requires full tree instance
- **Hard to optimize**: Can't focus on performance-critical parts

## Goal

Extract the core splitting logic into a pure, testable, optimizable function.

## Step-by-Step Plan

### Phase 1: Extract Pure Splitting Logic

**1. Create `split_leaf_items` function**
```rust
/// Pure function that determines how to split a full leaf
/// Input: existing items + new item to insert
/// Output: (left_items, right_items, separator_key)
fn split_leaf_items<K: Ord + Clone, V>(
    existing_items: &[(K, V)],
    new_key: K,
    new_value: V,
) -> (Vec<(K, V)>, Vec<(K, V)>, K)
```

**2. Write comprehensive tests**
- Test insertion at beginning, middle, end
- Test edge cases (1 item, even/odd splits)
- Test with different data types
- Benchmark the pure function

**3. Verify correctness**
- Ensure all items preserved
- Ensure proper ordering
- Ensure correct separator key

### Phase 2: Optimize the Pure Function

**4. Remove Vec allocation (the original performance issue)**
```rust
/// Optimized version that works with iterators/slices
/// No intermediate Vec allocation
fn split_leaf_items_optimized<K: Ord + Clone, V>(
    existing_items: &[(K, V)],
    new_key: K,
    new_value: V,
    left_output: &mut [(K, V)],
    right_output: &mut [(K, V)],
) -> (usize, usize, K)  // (left_count, right_count, separator)
```

**5. Benchmark the optimization**
- Compare Vec vs no-Vec versions
- Measure with different leaf sizes
- Ensure performance improvement

### Phase 3: Refactor the Tree Integration

**6. Create thin wrapper in `leaf_insert_or_split`**
```rust
unsafe fn leaf_insert_or_split(&mut self, leaf: NonNull<u8>, key: K, value: V) -> InsertResult<K, V> {
    // 1. Read current leaf items into slice/Vec
    let existing_items = self.read_leaf_items(leaf);
    
    // 2. Call pure splitting function
    let (left_items, right_items, sep_key) = split_leaf_items_optimized(
        &existing_items, key, value, ...
    );
    
    // 3. Handle memory allocation and tree state updates
    self.apply_leaf_split(leaf, left_items, right_items, sep_key)
}
```

**7. Extract helper methods**
- `read_leaf_items()` - Read leaf into slice
- `apply_leaf_split()` - Handle allocation and tree updates

### Phase 4: Validation and Cleanup

**8. Comprehensive testing**
- Run all existing tests
- Add integration tests
- Performance regression testing

**9. Benchmark end-to-end**
- Compare before/after performance
- Ensure no regressions in other operations

**10. Documentation and cleanup**
- Document the new architecture
- Remove old commented code
- Update performance tuning plan

## Expected Benefits

### Immediate Benefits
- **Testable**: Can test splitting logic in isolation
- **Understandable**: Clear separation of concerns
- **Debuggable**: Easier to isolate issues

### Performance Benefits
- **Optimizable**: Can focus on algorithm without side effects
- **No Vec allocation**: Direct memory operations
- **Better cache locality**: Fewer memory copies

### Long-term Benefits
- **Reusable**: Same logic for bulk operations
- **Maintainable**: Changes to algorithm don't affect tree management
- **Extensible**: Easy to add new splitting strategies

## Risk Mitigation

1. **Incremental approach**: Each phase can be validated independently
2. **Comprehensive testing**: Maintain 100% test success rate
3. **Performance monitoring**: Benchmark at each step
4. **Rollback plan**: Each phase can be reverted if needed

## Success Criteria

- [ ] Pure splitting function with 100% test coverage
- [ ] No Vec allocation in optimized version
- [ ] Performance improvement in insert operations
- [ ] All existing tests still pass
- [ ] Code is more readable and maintainable

## Next Steps

1. Start with Phase 1: Extract the pure function
2. Write tests to validate correctness
3. Proceed incrementally through each phase
4. Measure and validate at each step

This refactoring will unlock the ability to optimize the core performance bottleneck while improving code quality.
