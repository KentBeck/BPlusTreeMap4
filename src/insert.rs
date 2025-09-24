use alloc::vec::Vec;
use core::mem::ManuallyDrop;
use core::ptr::NonNull;

use crate::layout;
use crate::{alloc_branch_block, alloc_leaf_block, BPlusTreeMap, BTreeResult, NodeHdr, NodeTag};

pub(crate) enum InsertResult<K, V> {
    NoSplit(Option<V>),
    Split {
        sep_key: K,
        right: NonNull<u8>,
        old_value: Option<V>,
    },
}

impl<K: Ord + Clone, V> BPlusTreeMap<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let root = match self.root {
            Some(p) => p,
            None => unsafe { alloc_leaf_block(&self.leaf_layout).expect("alloc leaf") },
        };
        if self.root.is_none() {
            self.root = Some(root);
        }
        let res = unsafe { self.insert_rec(root, key, value) };
        match res {
            InsertResult::NoSplit(old) => old,
            InsertResult::Split {
                sep_key,
                right,
                old_value,
            } => {
                unsafe {
                    let branch =
                        alloc_branch_block(&self.branch_layout).expect("alloc new root branch");
                    let b = layout::carve_branch::<K>(branch, &self.branch_layout);
                    let bhdr = &mut *b.hdr;
                    bhdr.len = 1;
                    self.write_key_at(b.keys_ptr as *mut K, 0, sep_key);
                    let c0 = b.children_ptr as *mut *mut u8;
                    let c1 = c0.add(1);
                    *c0 = root.as_ptr();
                    *c1 = right.as_ptr();
                    self.root = Some(branch);
                }
                old_value
            }
        }
    }

    pub fn batch_insert(&mut self, items: Vec<(K, V)>) -> BTreeResult<Vec<Option<V>>> {
        let mut old_vals = Vec::with_capacity(items.len());
        for (k, v) in items {
            old_vals.push(self.insert(k, v));
        }
        Ok(old_vals)
    }

    unsafe fn insert_rec(&mut self, node: NonNull<u8>, key: K, value: V) -> InsertResult<K, V> {
        let hdr = &*(node.as_ptr() as *const NodeHdr);
        match hdr.tag {
            NodeTag::Leaf => self.leaf_insert_or_split(node, key, value),
            NodeTag::Branch => {
                let (child, child_idx) = self.child_for_key(node, &key).expect("child must exist");
                match self.insert_rec(child, key, value) {
                    InsertResult::NoSplit(old) => InsertResult::NoSplit(old),
                    InsertResult::Split {
                        sep_key,
                        right,
                        old_value,
                    } => {
                        let b = layout::carve_branch::<K>(node, &self.branch_layout);
                        let cur_len = (*b.hdr).len as usize;
                        let cap = self.branch_layout.cap as usize;
                        if cur_len < cap {
                            core::ptr::copy(
                                b.keys_ptr.add(child_idx) as *mut K,
                                b.keys_ptr.add(child_idx + 1) as *mut K,
                                cur_len - child_idx,
                            );
                            self.write_key_at(b.keys_ptr as *mut K, child_idx, sep_key);
                            let cbase = b.children_ptr as *mut *mut u8;
                            core::ptr::copy(
                                cbase.add(child_idx + 1),
                                cbase.add(child_idx + 2),
                                cur_len - child_idx,
                            );
                            *cbase.add(child_idx + 1) = right.as_ptr();
                            (*b.hdr).len = (cur_len + 1) as u16;
                            InsertResult::NoSplit(old_value)
                        } else {
                            self.branch_insert_and_split(node, child_idx, sep_key, right, old_value)
                        }
                    }
                }
            }
        }
    }

    unsafe fn branch_insert_and_split(
        &mut self,
        node: NonNull<u8>,
        insert_idx: usize,
        ins_key: K,
        ins_right: NonNull<u8>,
        old_value: Option<V>,
    ) -> InsertResult<K, V> {
        let b = layout::carve_branch::<K>(node, &self.branch_layout);
        let len = (*b.hdr).len as usize;
        let total_keys = len + 1;

        let mut keys_vec: Vec<K> = Vec::with_capacity(total_keys);
        for i in 0..len {
            keys_vec.push(core::ptr::read((b.keys_ptr as *const K).add(i)));
        }
        keys_vec.insert(insert_idx, ins_key);

        let total_children = total_keys + 1;
        let mut childs: Vec<*mut u8> = Vec::with_capacity(total_children);
        let cbase = b.children_ptr as *const *mut u8;
        for i in 0..=len {
            childs.push(*cbase.add(i));
        }
        childs.insert(insert_idx + 1, ins_right.as_ptr());

        let mid = total_keys / 2;
        let promote = core::ptr::read(keys_vec.as_ptr().add(mid));

        (*b.hdr).len = mid as u16;
        for i in 0..mid {
            self.write_key_at(
                b.keys_ptr as *mut K,
                i,
                core::ptr::read(keys_vec.as_ptr().add(i)),
            );
        }
        let cbase_mut = b.children_ptr as *mut *mut u8;
        for i in 0..=mid {
            *cbase_mut.add(i) = *childs.as_ptr().add(i);
        }

        let right_keys_len = total_keys - (mid + 1);
        let right_children_len = total_children - (mid + 1);
        let right_node = alloc_branch_block(&self.branch_layout).expect("alloc right branch");
        let rb = layout::carve_branch::<K>(right_node, &self.branch_layout);
        (*rb.hdr).len = right_keys_len as u16;
        for i in 0..right_keys_len {
            self.write_key_at(
                rb.keys_ptr as *mut K,
                i,
                core::ptr::read(keys_vec.as_ptr().add(mid + 1 + i)),
            );
        }
        let rcbase = rb.children_ptr as *mut *mut u8;
        for i in 0..right_children_len {
            *rcbase.add(i) = *childs.as_ptr().add(mid + 1 + i);
        }

        keys_vec.set_len(0);
        childs.set_len(0);

        InsertResult::Split {
            sep_key: promote,
            right: right_node,
            old_value,
        }
    }

    unsafe fn leaf_insert_or_split(
        &mut self,
        leaf: NonNull<u8>,
        key: K,
        value: V,
    ) -> InsertResult<K, V> {
        let parts = layout::carve_leaf::<K, V>(leaf, &self.leaf_layout);
        let hdr = &mut *parts.hdr;
        let len = hdr.len as usize;
        let keys = core::slice::from_raw_parts(parts.keys_ptr as *const K, len);
        match keys.binary_search(&key) {
            Ok(idx) => {
                let vptr = parts.vals_ptr.add(idx) as *mut V;
                let old = core::ptr::read(vptr);
                core::ptr::write(vptr, value);
                InsertResult::NoSplit(Some(old))
            }
            Err(idx) => {
                if len < self.leaf_layout.cap as usize {
                    self.shift_right(parts.keys_ptr as *mut K, parts.vals_ptr as *mut V, idx, len);
                    self.write_kv_at(
                        parts.keys_ptr as *mut K,
                        parts.vals_ptr as *mut V,
                        idx,
                        key,
                        value,
                    );
                    hdr.len = (len + 1) as u16;
                    self.len_count += 1;
                    InsertResult::NoSplit(None)
                } else {
                    let mut items_vec: Vec<(K, V)> = Vec::with_capacity(len + 1);
                    for i in 0..len {
                        let (k, v) = self.read_kv_at(
                            parts.keys_ptr as *const K,
                            parts.vals_ptr as *const V,
                            i,
                        );
                        items_vec.push((k, v));
                    }
                    let pos = items_vec
                        .binary_search_by(|(kk, _)| kk.cmp(&key))
                        .unwrap_or_else(|e| e);
                    items_vec.insert(pos, (key, value));
                    let total = items_vec.len();
                    let left_count = total / 2;
                    let right_count = total - left_count;
                    let mut items = ManuallyDrop::new(items_vec);
                    let base = items.as_mut_ptr();
                    let cap = items.capacity();
                    for i in 0..left_count {
                        let (kk, vv) = core::ptr::read(base.add(i));
                        self.write_kv_at(
                            parts.keys_ptr as *mut K,
                            parts.vals_ptr as *mut V,
                            i,
                            kk,
                            vv,
                        );
                    }
                    hdr.len = left_count as u16;

                    let right = alloc_leaf_block(&self.leaf_layout).expect("alloc right leaf");
                    let r = layout::carve_leaf::<K, V>(right, &self.leaf_layout);
                    (*r.hdr).len = right_count as u16;
                    for i in 0..right_count {
                        let (kk, vv) = core::ptr::read(base.add(left_count + i));
                        self.write_kv_at(r.keys_ptr as *mut K, r.vals_ptr as *mut V, i, kk, vv);
                    }
                    let _ = Vec::<(K, V)>::from_raw_parts(base, 0, cap);

                    let left_next = parts.next_ptr;
                    let old_next = *left_next;
                    *left_next = right.as_ptr();
                    if let Some(prev_ptr) = r.prev_ptr {
                        *prev_ptr = leaf.as_ptr();
                    }
                    let rnext = r.next_ptr;
                    *rnext = old_next;
                    if !old_next.is_null() {
                        if let Some(prev_off) = self.leaf_layout.prev_off {
                            let on_prev = (old_next.add(prev_off)) as *mut *mut u8;
                            *on_prev = right.as_ptr();
                        }
                    }

                    self.len_count += 1;
                    let sep = self.key_clone_at(r.keys_ptr as *const K, 0);
                    InsertResult::Split {
                        sep_key: sep,
                        right,
                        old_value: None,
                    }
                }
            }
        }
    }
}
