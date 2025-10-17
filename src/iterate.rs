use alloc::vec::IntoIter;
use alloc::vec::Vec;
use core::ops::{Bound, RangeBounds};
use core::ptr::NonNull;

use crate::layout;
use crate::{BPlusTreeMap, NodeHdr, NodeTag};

pub enum ItemsInner<'a, K, V> {
    Lazy {
        tree: &'a BPlusTreeMap<K, V>,
        front_leaf: Option<NonNull<u8>>,
        front_idx: usize,
        back_leaf: Option<NonNull<u8>>,
        back_idx: usize,
        remaining: usize,
        start_bound: Bound<K>,
        end_bound: Bound<K>,
        initialized: bool,
    },
    Vec {
        inner: IntoIter<(&'a K, &'a V)>,
    },
}

pub struct Items<'a, K, V> {
    pub(crate) inner: ItemsInner<'a, K, V>,
}

impl<'a, K: Ord, V> Iterator for Items<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            ItemsInner::Lazy {
                tree,
                front_leaf,
                front_idx,
                remaining,
                start_bound,
                end_bound,
                initialized,
                ..
            } => {
                // Lazy initialization on first call
                if !*initialized {
                    *initialized = true;
                    let is_excluded = matches!(start_bound, Bound::Excluded(_));
                    match start_bound {
                        Bound::Unbounded => {
                            *front_leaf = tree.leftmost_leaf();
                            *front_idx = 0;
                        }
                        Bound::Included(k) | Bound::Excluded(k) => {
                            let leaf_opt = tree.leaf_for_key(k);
                            if let Some(leaf) = leaf_opt {
                                unsafe {
                                    let parts = layout::carve_leaf::<K, V>(leaf, &tree.leaf_layout);
                                    let len = (*parts.hdr).len as usize;
                                    let keys = core::slice::from_raw_parts(
                                        parts.keys_ptr as *const K,
                                        len,
                                    );

                                    match keys.binary_search(k) {
                                        Ok(i) => {
                                            let idx = if is_excluded { i + 1 } else { i };
                                            if idx >= len {
                                                // Move to next leaf
                                                let next_ptr = *parts.next_ptr;
                                                *front_leaf = NonNull::new(next_ptr);
                                                *front_idx = 0;
                                            } else {
                                                *front_leaf = Some(leaf);
                                                *front_idx = idx;
                                            }
                                        }
                                        Err(i) => {
                                            if i >= len {
                                                // Move to next leaf
                                                let next_ptr = *parts.next_ptr;
                                                *front_leaf = NonNull::new(next_ptr);
                                                *front_idx = 0;
                                            } else {
                                                *front_leaf = Some(leaf);
                                                *front_idx = i;
                                            }
                                        }
                                    }
                                }
                            } else {
                                *front_leaf = None;
                                *front_idx = 0;
                            }
                        }
                    }
                }

                // Loop to handle leaf boundary crossing without recursion
                loop {
                    let leaf = (*front_leaf)?;
                    unsafe {
                        let parts = layout::carve_leaf::<K, V>(leaf, &tree.leaf_layout);
                        let len = (*parts.hdr).len as usize;

                        if *front_idx < len {
                            let k = &*(parts.keys_ptr.add(*front_idx) as *const K);

                            // Check end bound
                            let within_bound = match end_bound {
                                Bound::Unbounded => true,
                                Bound::Included(e) => k <= e,
                                Bound::Excluded(e) => k < e,
                            };

                            if !within_bound {
                                *front_leaf = None;
                                *remaining = 0;
                                return None;
                            }

                            let v = &*(parts.vals_ptr.add(*front_idx) as *const V);
                            *front_idx += 1;
                            if *remaining > 0 {
                                *remaining -= 1;
                            }
                            return Some((k, v));
                        }

                        // Move to next leaf
                        let next_ptr = *parts.next_ptr;
                        if next_ptr.is_null() {
                            *front_leaf = None;
                            *remaining = 0;
                            return None;
                        }

                        *front_leaf = NonNull::new(next_ptr);
                        *front_idx = 0;
                        // Continue loop instead of recursive call
                    }
                }
            }
            ItemsInner::Vec { inner } => inner.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            ItemsInner::Lazy {
                remaining,
                initialized,
                ..
            } => {
                if *initialized && *remaining > 0 {
                    (*remaining, Some(*remaining))
                } else {
                    // Uninitialized or unknown size
                    (0, None)
                }
            }
            ItemsInner::Vec { inner } => inner.size_hint(),
        }
    }
}

impl<'a, K: Ord, V> DoubleEndedIterator for Items<'a, K, V> {
    fn next_back(&mut self) -> Option<<Self as Iterator>::Item> {
        match &mut self.inner {
            ItemsInner::Lazy {
                tree,
                back_leaf,
                back_idx,
                remaining,
                start_bound,
                ..
            } => {
                let leaf = (*back_leaf)?;
                unsafe {
                    let parts = layout::carve_leaf::<K, V>(leaf, &tree.leaf_layout);

                    if *back_idx > 0 {
                        *back_idx -= 1;
                        let k = &*(parts.keys_ptr.add(*back_idx) as *const K);

                        // Check start bound
                        let within_bound = match start_bound {
                            Bound::Unbounded => true,
                            Bound::Included(s) => k >= s,
                            Bound::Excluded(s) => k > s,
                        };

                        if !within_bound {
                            *back_leaf = None;
                            *remaining = 0;
                            return None;
                        }

                        let v = &*(parts.vals_ptr.add(*back_idx) as *const V);
                        if *remaining > 0 {
                            *remaining -= 1;
                        }
                        return Some((k, v));
                    }

                    // Move to previous leaf
                    let prev_ptr = match parts.prev_ptr {
                        Some(p) => *p,
                        None => core::ptr::null_mut(),
                    };
                    if prev_ptr.is_null() {
                        *back_leaf = None;
                        *remaining = 0;
                        return None;
                    }

                    *back_leaf = NonNull::new(prev_ptr);
                    let prev_parts =
                        layout::carve_leaf::<K, V>(back_leaf.unwrap(), &tree.leaf_layout);
                    *back_idx = (*prev_parts.hdr).len as usize;
                    self.next_back()
                }
            }
            ItemsInner::Vec { inner } => inner.next_back(),
        }
    }
}

pub struct Keys<'a, K, V> {
    pub(crate) inner: Items<'a, K, V>,
}

impl<'a, K: Ord, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K: Ord, V> DoubleEndedIterator for Keys<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(k, _)| k)
    }
}

pub struct Values<'a, K, V> {
    pub(crate) inner: Items<'a, K, V>,
}

impl<'a, K: Ord, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K: Ord, V> DoubleEndedIterator for Values<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn items(&self) -> Items<'_, K, V> {
        let len = self.len();
        if len == 0 {
            return Items {
                inner: ItemsInner::Lazy {
                    tree: self,
                    front_leaf: None,
                    front_idx: 0,
                    back_leaf: None,
                    back_idx: 0,
                    remaining: 0,
                    start_bound: Bound::Unbounded,
                    end_bound: Bound::Unbounded,
                    initialized: true,
                },
            };
        }

        let front_leaf = self.leftmost_leaf();
        let back_leaf = self.rightmost_leaf();
        let back_idx = if let Some(leaf) = back_leaf {
            unsafe {
                let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
                (*parts.hdr).len as usize
            }
        } else {
            0
        };

        Items {
            inner: ItemsInner::Lazy {
                tree: self,
                front_leaf,
                front_idx: 0,
                back_leaf,
                back_idx,
                remaining: len,
                start_bound: Bound::Unbounded,
                end_bound: Bound::Unbounded,
                initialized: true,
            },
        }
    }

    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys {
            inner: self.items(),
        }
    }

    pub fn values(&self) -> Values<'_, K, V> {
        Values {
            inner: self.items(),
        }
    }

    pub fn items_range(&self, start: Option<&K>, end: Option<&K>) -> Items<'_, K, V> {
        // TODO: Implement lazy range iteration
        // For now, collect into Vec (old implementation)
        let sb = start.map_or(Bound::Unbounded, Bound::Included);
        let eb = end.map_or(Bound::Unbounded, Bound::Excluded);
        Items {
            inner: ItemsInner::Vec {
                inner: self.collect_range_bounds(sb, eb).into_iter(),
            },
        }
    }

    pub fn range<R: RangeBounds<K>>(&self, r: R) -> Items<'_, K, V> {
        let start_bound = r.start_bound();
        let end_bound = r.end_bound();

        Items {
            inner: ItemsInner::Lazy {
                tree: self,
                front_leaf: None,
                front_idx: 0,
                back_leaf: None,
                back_idx: 0,
                remaining: 0, // Unknown for ranges, size_hint will return (0, None)
                start_bound: Self::clone_bound(start_bound),
                end_bound: Self::clone_bound(end_bound),
                initialized: false,
            },
        }
    }

    fn clone_bound(bound: Bound<&K>) -> Bound<K> {
        match bound {
            Bound::Unbounded => Bound::Unbounded,
            Bound::Included(k) => Bound::Included(k.clone()),
            Bound::Excluded(k) => Bound::Excluded(k.clone()),
        }
    }

    pub fn first(&self) -> Option<(&K, &V)> {
        self.items().next()
    }

    pub fn last(&self) -> Option<(&K, &V)> {
        self.items().last()
    }

    pub(crate) fn collect_range_bounds<'a>(
        &'a self,
        start: Bound<&K>,
        end: Bound<&K>,
    ) -> Vec<(&'a K, &'a V)> {
        let mut out = Vec::new();
        let leaf_ptr = match start {
            Bound::Unbounded => self.leftmost_leaf(),
            Bound::Included(k) | Bound::Excluded(k) => self.leaf_for_key(k),
        };
        if leaf_ptr.is_none() {
            return out;
        }
        unsafe {
            let mut cur = leaf_ptr.unwrap().as_ptr();
            let mut first_idx = 0usize;
            if let Bound::Included(s) | Bound::Excluded(s) = start {
                let parts =
                    layout::carve_leaf::<K, V>(NonNull::new_unchecked(cur), &self.leaf_layout);
                let len = (*parts.hdr).len as usize;
                let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
                match self.binary_search_keys(keys, s) {
                    Ok(i) => {
                        first_idx = if matches!(start, Bound::Excluded(_)) {
                            i + 1
                        } else {
                            i
                        };
                    }
                    Err(i) => {
                        first_idx = i;
                    }
                }
            }
            loop {
                if cur.is_null() {
                    break;
                }
                let hdr = &*(cur as *const NodeHdr);
                if hdr.tag != NodeTag::Leaf {
                    break;
                }
                let len = hdr.len as usize;
                let keys_ptr = (cur.add(self.leaf_layout.keys_off)) as *const K;
                let vals_ptr = (cur.add(self.leaf_layout.vals_off)) as *const V;
                for i in first_idx..len {
                    let kref = &*keys_ptr.add(i);
                    let end_ok = match end {
                        Bound::Unbounded => true,
                        Bound::Included(e) => kref <= e,
                        Bound::Excluded(e) => kref < e,
                    };
                    if !end_ok {
                        return out;
                    }
                    let vref = &*vals_ptr.add(i);
                    out.push((kref, vref));
                }
                first_idx = 0;
                let next_ptr = (cur.add(self.leaf_layout.next_off)) as *const *mut u8;
                cur = *next_ptr;
            }
        }
        out
    }
}
