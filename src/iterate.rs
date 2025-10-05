use alloc::vec::IntoIter;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ops::{Bound, RangeBounds};
use core::ptr::NonNull;

use crate::layout;
use crate::{BPlusTreeMap, NodeHdr, NodeTag};

pub struct Items<'a, K, V> {
    pub(crate) inner: IntoIter<(&'a K, &'a V)>,
}

impl<'a, K, V> Iterator for Items<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, K, V> DoubleEndedIterator for Items<'a, K, V> {
    fn next_back(&mut self) -> Option<<Self as Iterator>::Item> {
        self.inner.next_back()
    }
}

pub struct Keys<'a, K, V> {
    pub(crate) inner: IntoIter<&'a K>,
    pub(crate) _marker: PhantomData<V>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

pub struct Values<'a, K, V> {
    pub(crate) inner: IntoIter<&'a V>,
    pub(crate) _marker: PhantomData<K>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn items(&self) -> Items<'_, K, V> {
        Items {
            inner: self
                .collect_range_bounds(Bound::Unbounded, Bound::Unbounded)
                .into_iter(),
        }
    }

    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys::<K, V> {
            inner: self.items().map(|(k, _)| k).collect::<Vec<_>>().into_iter(),
            _marker: PhantomData,
        }
    }

    pub fn values(&self) -> Values<'_, K, V> {
        Values::<K, V> {
            inner: self.items().map(|(_, v)| v).collect::<Vec<_>>().into_iter(),
            _marker: PhantomData,
        }
    }

    pub fn items_range(&self, start: Option<&K>, end: Option<&K>) -> Items<'_, K, V> {
        let sb = start.map_or(Bound::Unbounded, Bound::Included);
        let eb = end.map_or(Bound::Unbounded, Bound::Excluded);
        Items {
            inner: self.collect_range_bounds(sb, eb).into_iter(),
        }
    }

    pub fn range<R: RangeBounds<K>>(&self, r: R) -> Items<'_, K, V> {
        Items {
            inner: self
                .collect_range_bounds(r.start_bound(), r.end_bound())
                .into_iter(),
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
