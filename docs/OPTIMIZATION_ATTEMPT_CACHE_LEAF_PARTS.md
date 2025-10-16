# Failed Optimization: Cache Leaf Parts in Range Iterator

## Hypothesis
Assembly analysis showed that `carve_leaf()` operations were not being optimized away by the compiler. Each iteration was loading layout offsets from memory:

```assembly
mov rdx, qword ptr [rsi + 56]  # Load vals_off
add rdx, rax                    # Compute vals_ptr
add rax, qword ptr [rsi + 48]  # Load keys_off
```

Expected savings: 3-4 cycles per element (35-40% of total cost)

## Implementation
Added three cached fields to `ItemsInner::Lazy`:
- `cached_keys_ptr: *const K`
- `cached_vals_ptr: *const V`
- `cached_len: usize`

Modified hot path to use cached pointers instead of calling `carve_leaf()` every iteration.

## Results
**Performance regression of ~10%:**

| Metric | Baseline | Optimized | Change |
|--------|----------|-----------|--------|
| Cycles per element | 10 | 11 | +10% ❌ |
| 100-element range | 4.614ms | 4.972ms | +7.8% ❌ |

## Analysis
The optimization made performance worse despite assembly showing clear overhead. Possible reasons:

1. **Increased struct size**: Adding 3 fields (24 bytes) to `ItemsInner::Lazy` may have:
   - Caused cache line misses
   - Increased memory pressure
   - Affected struct alignment

2. **Additional memory loads**: Accessing cached fields requires loading them from memory, potentially offsetting any savings from avoiding `carve_leaf()` computation

3. **Compiler was already optimizing**: The compiler may have been doing better optimization than the assembly suggested (e.g., register allocation, instruction reordering)

4. **Disrupted CPU optimizations**: Changes may have affected:
   - Branch prediction patterns
   - Hardware prefetcher behavior
   - Instruction-level parallelism

## Conclusion
**Optimization reverted.** The assembly analysis correctly identified overhead, but caching the values made performance worse rather than better. This suggests the bottleneck is elsewhere or that the struct size increase outweighed the computational savings.

## Lessons Learned
- Assembly analysis alone is insufficient - must measure actual performance
- Struct size and cache locality matter more than individual instruction counts
- Modern CPUs have complex optimization behaviors that can be disrupted by seemingly beneficial changes
- Always benchmark before and after, even when theory suggests improvement
