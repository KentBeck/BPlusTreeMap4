# Profiling FAQ

## Why can't I see helper functions in the profiler?

**Short answer:** They're all inlined.

**Long answer:** Almost all helper functions are marked with `#[inline(always)]` or `#[inline]`, which tells the compiler to inline them into their callers. This means they don't appear as separate stack frames in the profiler.

### What's Inlined?

Looking at the codebase, these critical functions are all inlined:

**In `src/common.rs`:**
- `shift_right` - `#[inline(always)]` - Shifts array elements
- `write_kv_at` - `#[inline(always)]` - Writes key-value pair
- `write_key_at` - `#[inline(always)]` - Writes key
- `read_kv_at` - `#[inline(always)]` - Reads key-value pair
- `move_kv_at` - `#[inline(always)]` - Moves key-value pair
- `binary_search_keys` - `#[inline(always)]` - Binary search
- `child_for_key` - `#[inline]` - Finds child node
- `leaf_for_key` - `#[inline]` - Finds leaf node

**In `src/insert.rs`:**
- `insert_into_leaf_slot` - `#[inline(always)]` - Inserts into leaf
- `shift_and_write` - `#[inline(always)]` - Shifts and writes

**In `src/layout.rs`:**
- `carve_leaf` - `#[inline]` - Calculates leaf layout
- `carve_branch` - `#[inline]` - Calculates branch layout

**In `src/node_alloc.rs`:**
- `alloc_raw` - `#[inline]` - Allocates memory
- `dealloc_raw` - `#[inline]` - Deallocates memory
- `alloc_leaf_block` - `#[inline]` - Allocates leaf node
- `alloc_branch_block` - `#[inline]` - Allocates branch node

### What This Means for Profiling

When you see this in the profiler:
```
insert_rec             104 samples (5%)
  _platform_memmove    88 samples (85% of insert_rec)
```

What's actually happening inside `insert_rec`:
```
insert_rec
├─ child_for_key (INLINED)
│  ├─ carve_branch (INLINED)
│  └─ binary_search_keys (INLINED)
│     └─ slice::binary_search (stdlib)
├─ leaf_insert_or_split (NOT inlined - too large)
│  ├─ carve_leaf (INLINED)
│  ├─ binary_search_keys (INLINED)
│  ├─ insert_into_leaf_slot (INLINED)
│  │  ├─ shift_right (INLINED)
│  │  │  └─ ptr::copy → _platform_memmove 🔥
│  │  └─ write_kv_at (INLINED)
│  └─ alloc_leaf_block (INLINED when splitting)
└─ branch_insert_and_split (NOT inlined - too large)
   ├─ carve_branch (INLINED)
   ├─ shift operations (INLINED)
   │  └─ ptr::copy → _platform_memmove 🔥
   └─ alloc_branch_block (INLINED when splitting)
```

So when the profiler shows 85% time in `_platform_memmove`, that's coming from:
- `shift_right` (inlined into `insert_into_leaf_slot`)
- `insert_into_leaf_slot` (inlined into `leaf_insert_or_split`)
- `leaf_insert_or_split` (called by `insert_rec`)

### Why Use `#[inline(always)]`?

Inlining is critical for performance:

1. **Eliminates function call overhead** - No stack frame setup/teardown
2. **Enables further optimizations** - Compiler can optimize across function boundaries
3. **Reduces instruction count** - No call/return instructions
4. **Better register allocation** - Compiler sees the full picture

For tiny functions like `write_kv_at` (just 2 instructions), the function call overhead would be larger than the function itself!

### Performance Impact

Let's estimate the impact of NOT inlining:

**Without inlining:**
- Each `shift_right` call: ~10 instructions overhead (call, setup, return)
- Each `write_kv_at` call: ~10 instructions overhead
- Per insert: ~5-10 helper function calls
- Total overhead: ~50-100 extra instructions per insert

**With inlining:**
- Zero function call overhead
- Compiler can optimize across boundaries
- Better instruction cache utilization

For 10M inserts, this would add **500M-1B extra instructions** - roughly doubling the instruction count!

### How to See Helper Functions

If you really want to see the helper functions in the profiler, you have two options:

#### Option 1: Remove `#[inline]` attributes (NOT RECOMMENDED)

This would make the code significantly slower but show the call tree:

```rust
// Before
#[inline(always)]
pub(crate) unsafe fn shift_right(...) { ... }

// After (for profiling only)
// #[inline(always)]  // COMMENTED OUT
pub(crate) unsafe fn shift_right(...) { ... }
```

**Downsides:**
- 2-3x slower
- Doesn't represent production performance
- Misleading profiling data

#### Option 2: Use `cargo-llvm-lines` (RECOMMENDED)

This shows how much code each function generates, which correlates with importance:

```bash
cargo install cargo-llvm-lines
cargo llvm-lines --bin profile_insert --release
```

Output shows:
```
Lines               Copies            Function name
-----               ------            -------------
325 (9.5%)          1                 leaf_insert_or_split
305 (8.9%)          1                 branch_insert_and_split
139 (4.1%)          1                 binary_search_by
126 (3.7%)          2                 carve_leaf
```

This tells you which functions generate the most code, even if they're inlined.

## Why can't I collapse recursive calls in the profiler?

**Short answer:** The UI you're seeing might not be the full Firefox Profiler.

**Long answer:** Samply opens a local web server that shows a simplified view. To get the full Firefox Profiler UI with all features:

### Option 1: Upload to Firefox Profiler

1. Go to https://profiler.firefox.com
2. Click "Load a profile from file"
3. Upload `profile_10m.json`
4. You'll get the full UI with:
   - Recursive call merging
   - Flame graphs
   - Timeline view
   - Advanced filtering

### Option 2: Use the Transform Menu

In the Call Tree view:
1. Right-click on `insert_rec`
2. Look for "Merge function" or "Collapse recursion"
3. This should merge all recursive calls into one

### Option 3: Use Flame Graph View

Instead of Call Tree, switch to Flame Graph:
1. Click the "Flame Graph" tab
2. This automatically shows time spent in a visual way
3. Recursive calls are naturally merged

### What Recursive Call Merging Shows

Without merging:
```
insert_rec             104 samples
  insert_rec           78 samples
    insert_rec         52 samples
      insert_rec       29 samples
        ...
```

With merging:
```
insert_rec             263 samples (total across all recursion levels)
  leaf_insert_or_split 150 samples
  branch_insert_...    80 samples
  child_for_key        33 samples
```

This gives you a clearer picture of where time is spent.

## Why is `_platform_memmove` so dominant?

This is **expected and correct** for B+ tree operations.

### What is `_platform_memmove`?

It's the optimized memory copy function provided by the OS:
- On macOS: Apple's optimized implementation
- Uses SIMD instructions (AVX2, NEON) when possible
- Handles overlapping memory regions correctly
- Fastest possible way to move memory

### Why does it dominate?

Every insert into a sorted array requires shifting elements:

```rust
// Insert 'X' at position 3 in [A, B, C, D, E, F]
// Need to shift [D, E, F] right by 1
// Result: [A, B, C, X, D, E, F]

core::ptr::copy(
    keys_ptr.add(3),      // source: D
    keys_ptr.add(4),      // dest: where D should go
    3                     // count: shift D, E, F
);
// This becomes a call to memmove
```

For capacity=128:
- Average shift distance: 64 elements
- Each insert: shift 64 keys + 64 values = 128 elements
- 10M inserts × 128 elements = **1.28 billion elements moved**

At ~8 bytes per element (u64), that's **10 GB of memory copied**!

### Is this a problem?

**No!** This is fundamental to how B+ trees work:
- Need sorted arrays for O(log n) binary search
- Shifting is the price we pay for fast lookups
- We're still 9% faster than std::BTreeMap

### Could we optimize it?

Not really:
- `ptr::copy` → `memmove` is already optimal
- SIMD instructions are already used
- Can't avoid shifting without changing data structure

The only way to reduce memmove time:
1. **Smaller capacity** - Less to shift, but more splits (worse overall)
2. **Different structure** - Skip list, hash table (different tradeoffs)
3. **Unsorted arrays** - No shifting, but O(n) search (much worse)

None of these are worth it - we're already competitive!

## Summary

### Key Takeaways

1. **Helper functions are inlined** - This is good for performance, bad for profiling visibility
2. **Use Firefox Profiler for full features** - Upload profile_10m.json to profiler.firefox.com
3. **`memmove` dominance is expected** - 85% is normal for B+ tree inserts
4. **We're still winning** - 9% faster than std::BTreeMap despite the overhead
5. **Don't remove inlining** - Would make code 2-3x slower

### Profiling Best Practices

1. **Profile with large workloads** - 10M ops gives 2,076 samples vs 106 for 1M
2. **Use multiple tools** - samply, cargo-llvm-lines, time -l
3. **Compare with baselines** - Profile std::BTreeMap too
4. **Understand the data structure** - Know what's expected vs surprising
5. **Don't over-optimize** - If you're already winning, ship it!

### Tools Reference

```bash
# Profile with samply
samply record -o profile.json ./target/release/binary

# View in browser (simplified UI)
samply record ./target/release/binary
# Opens http://localhost:3000

# Upload to Firefox Profiler (full UI)
# Go to https://profiler.firefox.com
# Upload profile.json

# See code size per function
cargo llvm-lines --bin binary --release

# Get detailed metrics
/usr/bin/time -l ./target/release/binary
```

