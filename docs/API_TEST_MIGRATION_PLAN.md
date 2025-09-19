API Test Migration Plan

Goal
- Import the public API tests from BPlusTree3 into this repo, get them compiling against the new raw‑memory B+ tree, then iteratively make them pass.

Constraints
- New design: raw, single‑allocation nodes; no arena; no parent pointers.
- Keep crate API compatible enough that the prior tests compile (even if functions are initially stubs).
- Doubly‑linked leaves for reverse iteration.

High‑Level Phases
1) Crate Scaffolding
   - Add `Cargo.toml` with package name `bplustree` (matches prior tests’ `use bplustree::...`).
   - Keep `#![no_std]` + `alloc`; tests run under `std`.
   - Expose a public facade module for the API surface referenced by tests.

2) Import API Tests
   - Copy API‑facing tests from `vendor/BPlusTree3/rust/tests/` into this repo’s `tests/`.
   - Include `tests/test_utils.rs` (used by many tests).
   - Defer or gate arena‑specific tests/helpers (e.g., arena stats, manual node allocation) behind a feature flag to keep compilation green.

3) Compile‑Only Compatibility (Stubs)
   - Implement public types and signatures to satisfy the tests, backed by placeholders:
     - `pub struct BPlusTreeMap<K,V>`
     - `impl<K: Ord, V> BPlusTreeMap<K,V> { new(cap: usize) -> Result<Self, BPlusTreeError>; insert, get, get_mut, remove, contains_key, get_or_default, is_empty, is_leaf_root, leaf_count, items, items_range, keys, values, range, check_invariants, check_invariants_detailed }`
     - `pub enum BPlusTreeError { InvalidCapacity }` (expand as needed)
     - `pub enum NodeRef<K,V> { Leaf(u32, core::marker::PhantomData<(K,V)>), Branch(u32, core::marker::PhantomData<(K,V)>) }` with `id()`/`is_leaf()` for tests that exercise it.
   - Provide iterator types (`Items`, `ItemsRange`, etc.) that implement `Iterator` and `DoubleEndedIterator`, with `next()`/`next_back()` temporarily `unimplemented!()` or returning `None` so tests compile.
   - Make invariants checks return `true`/`Ok(())` temporarily to enable compilation.

4) Turn on Test Compilation
   - `cargo test --no-run` to ensure all tests compile.
   - If some tests still fail to compile due to arena‑specific APIs (e.g., `leaf_arena_stats`, `branch_arena_stats`, `allocate_leaf`), either:
     - Add no‑op shims on `BPlusTreeMap` guarded by a `#[cfg(feature = "compat_arena")]` feature that always returns defaults, or
     - Add `#[cfg(feature = "arena")]` gates to the imported tests where reasonable.

5) Gradual Test Enablement (Make Them Pass)
   - Implement core functionality in this order to turn failing tests green:
     1. Leaf‑root only: `insert`, `get`, `contains_key`, `remove` without splits; `len()` (on‑demand scan or add a counter).
     2. Leaf split + root promotion to branch; maintain `next/prev` links.
     3. Branch descent (search) + insertion that splits leaves and propagates a separator upward.
     4. Branch splits; multi‑level propagation.
     5. Deletion with borrow/merge (leaves), then branch fix‑ups.
     6. Iteration APIs: `items()`, `keys()`, `values()`; `items_range()` with `DoubleEndedIterator`.
     7. Invariant checks: validate ordering, capacities, leaf linking; implement `check_invariants`/`check_invariants_detailed`.

6) Clean Up Compatibility Layer
   - Remove or gate any temporary shims (arena compatibility) so they don’t affect production builds.
   - Keep `NodeRef` only if tests or public API require it; otherwise move it under a `testing` feature.

Deliverables Checklist
- [ ] Cargo.toml with `name = "bplustree"` and dev‑dependencies for tests if needed.
- [ ] Public API façade with stubs sufficient for compilation.
- [ ] Imported tests under `tests/` (API‑focused first). Arena‑centric tests gated or deferred.
- [ ] CI step: `cargo test --no-run` for compile‑only validation.
- [ ] Iterative implementation to flip tests to passing by feature area.

Notes
- Prior tests assume `len()` and many introspection helpers; we will initially stub these. If runtime panics are undesirable during early compile checks, have iterators return empty and invariants return success until their implementations land.
- Cache‑line sized nodes are already supported via `with_cache_lines`; tests using `BPlusTreeMap::new(cap)` will map to a simple constructor that derives byte budgets from `cap` (e.g., choose a default bytes budget and compute capacities, or invert cap into bytes). We will provide a `new(cap)` wrapper that configures layouts accordingly for test parity.

