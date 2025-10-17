# Investigation Plan: leftmost_leaf() Performance

## The Problem Statement

Benchmark shows "From Beginning" scenario is 5,000x slower than std::BTreeMap:
- BPlusTreeMap: 38-40ms for iterating first 100 items
- std::BTreeMap: 0.007ms for same operation
- Tree size: 10M items, capacity 128

Initial hypothesis was that `leftmost_leaf()` traversal is expensive. This makes NO SENSE.

## Why Initial Analysis Was Wrong

For a tree with 10M items and capacity 128:

**Tree Structure:**
- Leaf nodes: 10,000,000 / 128 = 78,125 leaves
- Level 1 branches: 78,125 / 128 = 611 nodes
- Level 2 branches: 611 / 128 = 5 nodes  
- Level 3 (root): 1 node
- **Total height: 4 levels**

**Cost of leftmost_leaf():**
```rust
pub(crate) fn leftmost_leaf(&self) -> Option<NonNull<u8>> {
    let mut cur = self.root?;
    unsafe {
        loop {
            let hdr = &*(cur.as_ptr() as *const NodeHdr);
            match hdr.tag {
                NodeTag::Leaf => return Some(cur),
                NodeTag::Branch => {
                    let b = layout::carve_branch::<K>(cur, &self.branch_layout);
                    let child_ptr = *(b.children_ptr as *const *mut u8);
                    cur = NonNull::new_unchecked(child_ptr);
                }
            }
        }
    }
}
```

**Operations per level:**
1. Dereference pointer to read header (1 cache line)
2. Check tag field (already in cache)
3. Call carve_branch() - compute offsets (arithmetic, no memory access)
4. Read first child pointer (already in cache from carve_branch)
5. Follow pointer to next level

**Expected cost:**
- 4 levels × ~10-20ns per level = **40-80ns total**
- NOT 15ms (which is 150,000,000ns - over 1 million times slower!)

## What We're Actually Measuring

Looking at the benchmark code:

```rust
fn bench_partial_iter_begin<M>(map: &M, count: usize) -> Duration {
    let start = Instant::now();
    let mut iter = map.iter();
    let mut n = 0;
    for (k, v) in iter.by_ref().take(count) {
        black_box((k, v));
        n += 1;
    }
    let elapsed = start.elapsed();
    black_box(n);
    elapsed
}
```

This measures ONE iteration through 100 items. The 38-40ms must include something else.

## Investigation Steps

### Step 1: Isolate leftmost_leaf() Cost

**Create micro-benchmark:**
```rust
fn main() {
    let mut map = BPlusTreeMap::new(128).unwrap();
    
    // Build 10M item tree
    for i in 0..10_000_000 {
        map.insert(i, i * 2);
    }
    
    // Measure ONLY leftmost_leaf() calls
    let start = Instant::now();
    for _ in 0..1_000_000 {
        let leaf = map.leftmost_leaf();
        black_box(leaf);
    }
    let elapsed = start.elapsed();
    
    println!("1M leftmost_leaf() calls: {:?}", elapsed);
    println!("Per call: {}ns", elapsed.as_nanos() / 1_000_000);
}
```

**Expected result:** 50-100ns per call
**If slower:** Something is wrong with carve_branch() or memory layout

### Step 2: Profile items() Iterator Creation

**Measure just iterator creation:**
```rust
fn main() {
    let mut map = BPlusTreeMap::new(128).unwrap();
    for i in 0..10_000_000 {
        map.insert(i, i * 2);
    }
    
    // Measure iterator creation WITHOUT consuming it
    let start = Instant::now();
    for _ in 0..100_000 {
        let iter = map.items();
        black_box(&iter);
        drop(iter);
    }
    let elapsed = start.elapsed();
    
    println!("100K items() calls: {:?}", elapsed);
    println!("Per call: {}ns", elapsed.as_nanos() / 100_000);
}
```

**Expected result:** 200-500ns per call (includes leftmost + rightmost + allocation)
**If slower:** Problem is in Items struct initialization, not leftmost_leaf()

### Step 3: Profile Actual Iteration

**Measure iteration separate from creation:**
```rust
fn main() {
    let mut map = BPlusTreeMap::new(128).unwrap();
    for i in 0..10_000_000 {
        map.insert(i, i * 2);
    }
    
    // Measure iteration of first 100 items (create iterator once)
    let start = Instant::now();
    for _ in 0..100_000 {
        let mut count = 0;
        for (k, v) in map.items() {
            black_box((k, v));
            count += 1;
            if count >= 100 {
                break;
            }
        }
    }
    let elapsed = start.elapsed();
    
    println!("100K iterations (100 items each): {:?}", elapsed);
    println!("Per iteration: {}us", elapsed.as_micros() / 100_000);
}
```

**Expected result:** 5-10μs per iteration
**If 38ms:** Problem is in the iteration logic itself

### Step 4: Compare with range()

**Test if range() is actually faster:**
```rust
fn main() {
    let mut map = BPlusTreeMap::new(128).unwrap();
    for i in 0..10_000_000 {
        map.insert(i, i * 2);
    }
    
    // Get first key
    let first_key = *map.items().next().unwrap().0;
    
    // Measure range(first_key..) vs items()
    let start = Instant::now();
    for _ in 0..100_000 {
        let mut count = 0;
        for (k, v) in map.range(first_key..) {
            black_box((k, v));
            count += 1;
            if count >= 100 {
                break;
            }
        }
    }
    let elapsed_range = start.elapsed();
    
    let start = Instant::now();
    for _ in 0..100_000 {
        let mut count = 0;
        for (k, v) in map.items() {
            black_box((k, v));
            count += 1;
            if count >= 100 {
                break;
            }
        }
    }
    let elapsed_items = start.elapsed();
    
    println!("range(): {:?}", elapsed_range);
    println!("items(): {:?}", elapsed_items);
    println!("Difference: {:?}", elapsed_items.saturating_sub(elapsed_range));
}
```

**Expected result:** Similar performance if leftmost_leaf() is ~100ns
**If big difference:** Need to understand why

### Step 5: Check Tree Building Time

**Verify the 38ms isn't from tree construction:**
```rust
fn main() {
    // Build tree
    let build_start = Instant::now();
    let mut map = BPlusTreeMap::new(128).unwrap();
    for i in 0..10_000_000 {
        map.insert(i, i * 2);
    }
    println!("Tree build: {:?}", build_start.elapsed());
    
    // Measure iteration
    let iter_start = Instant::now();
    let mut count = 0;
    for (k, v) in map.items() {
        black_box((k, v));
        count += 1;
        if count >= 100 {
            break;
        }
    }
    println!("First 100 items: {:?}", iter_start.elapsed());
}
```

### Step 6: Profile with Instruments (Properly)

**Create minimal repro:**
```rust
fn main() {
    let mut map = BPlusTreeMap::new(128).unwrap();
    for i in 0..10_000_000 {
        map.insert(i, i * 2);
    }
    
    println!("Tree built. Starting profiling...");
    std::thread::sleep(Duration::from_secs(5)); // Time to attach profiler
    
    // Repeat the EXACT benchmark operation 1000 times
    for _ in 0..1000 {
        let start = Instant::now();
        let mut iter = map.items();
        let mut n = 0;
        for (k, v) in iter.by_ref().take(100) {
            black_box((k, v));
            n += 1;
        }
        let elapsed = start.elapsed();
        black_box(n);
        
        // Print if it's actually slow
        if elapsed.as_millis() > 1 {
            println!("Slow iteration detected: {:?}", elapsed);
        }
    }
}
```

## Hypotheses to Test

### Hypothesis 1: Measurement Error
The benchmark is measuring something other than iteration cost.
**Test:** Step 5 - separate tree building from iteration

### Hypothesis 2: Memory Allocation
Items iterator allocates memory on creation.
**Test:** Step 2 - measure just iterator creation

### Hypothesis 3: Vec Collection
Maybe Items::Lazy is falling back to Vec somehow?
**Test:** Add debug prints to see which path is taken

### Hypothesis 4: carve_leaf() Called Repeatedly
Maybe carve_leaf() is called for every item, not cached?
**Test:** Add counters to carve_leaf() calls

### Hypothesis 5: Tree Structure Issue
Maybe the tree is malformed or unbalanced?
**Test:** Print tree stats (height, node counts, etc.)

### Hypothesis 6: The Benchmark Is Wrong
Maybe bench_partial_iter is not measuring what we think?
**Test:** Reproduce with manual timing code

## Expected Timeline

1. Step 1-3: 30 minutes (write and run micro-benchmarks)
2. Step 4-5: 15 minutes (comparison tests)
3. Step 6: 30 minutes (proper profiling)
4. Analysis: 30 minutes
5. Root cause identification: Should be obvious after steps 1-3

## Success Criteria

By the end of this investigation, we should know:
1. Exact cost of leftmost_leaf() in nanoseconds
2. Exact cost of items() creation in nanoseconds  
3. Exact cost of iterating 100 items in microseconds
4. Where the 38ms is actually coming from
5. Why "From Beginning" is slower than other scenarios

## Next Steps After Investigation

Once we know the real problem:
1. Fix the actual bottleneck
2. Re-run benchmarks to verify improvement
3. Update documentation with accurate analysis
4. Consider if caching is still beneficial

## Notes

- leftmost_leaf() should be 4 pointer dereferences = ~40-80ns
- 15ms = 150,000,000ns is 1,875,000 times slower than expected
- This can't be right - need to find what's really slow
- My previous analysis jumped to conclusions without proper measurement