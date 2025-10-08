use core::mem::MaybeUninit;
use core::mem::{align_of, size_of};
use core::ptr::NonNull;

#[inline]
pub const fn align_up(x: usize, a: usize) -> usize {
    debug_assert!(a.is_power_of_two());
    (x + (a - 1)) & !(a - 1)
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeTag {
    Branch = 0,
    Leaf = 1,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct NodeHdr {
    pub tag: NodeTag, // 1 byte
    pub len: u16,     // number of initialized keys in this node
    pub flags: u8,    // reserved
}

#[derive(Copy, Clone, Debug)]
pub struct LeafLayout {
    pub bytes: usize,
    pub cap: u16,
    pub max_align: usize,
    pub hdr_size: usize,
    // sibling pointers
    pub next_off: usize,
    pub prev_off: Option<usize>,
    // arrays
    pub keys_off: usize,
    pub vals_off: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct BranchLayout {
    pub bytes: usize,
    pub cap: u16,
    pub max_align: usize,
    pub hdr_size: usize,
    pub children_off: usize, // [*mut NodeHdr; cap+1]
    pub keys_off: usize,     // [K; cap]
}

impl LeafLayout {
    /// Compute a leaf layout for key type K and value type V.
    /// If `doubly_linked` is true, include space for both next and prev pointers.
    pub fn compute<K, V>(bytes: usize, doubly_linked: bool) -> Self {
        let a_ptr = align_of::<*const ()>();
        let a_k = align_of::<K>();
        let a_v = align_of::<V>();
        let s_ptr = size_of::<*const ()>();
        let s_k = size_of::<K>();
        let s_v = size_of::<V>();

        let max_align = a_ptr.max(a_k).max(a_v).max(align_of::<NodeHdr>());
        let hdr_size = align_up(size_of::<NodeHdr>(), max_align);

        let sib_bytes = if doubly_linked { 2 * s_ptr } else { s_ptr };
        let sib_off = align_up(hdr_size, a_ptr);
        let after_sib = sib_off + sib_bytes;

        // quick upper bound, ignoring alignment between arrays
        let mut cap_guess = bytes.saturating_sub(after_sib) / (s_k + s_v);
        if cap_guess > u16::MAX as usize {
            cap_guess = u16::MAX as usize;
        }

        let mut best = Self {
            bytes,
            cap: 0,
            max_align,
            hdr_size,
            next_off: sib_off,
            prev_off: if doubly_linked {
                Some(sib_off + s_ptr)
            } else {
                None
            },
            keys_off: after_sib,
            vals_off: after_sib,
        };

        while cap_guess > 0 {
            let first_is_keys = a_k >= a_v;
            let (a1, s1, a2, s2) = if first_is_keys {
                (a_k, s_k, a_v, s_v)
            } else {
                (a_v, s_v, a_k, s_k)
            };

            let first_off = align_up(after_sib, a1);
            let second_off = align_up(first_off + cap_guess * s1, a2);
            let end = second_off + cap_guess * s2;
            let end_aligned = align_up(end, max_align);

            if end_aligned <= bytes {
                best.cap = cap_guess as u16;
                let (keys_off, vals_off) = if first_is_keys {
                    (first_off, second_off)
                } else {
                    (second_off, first_off)
                };
                best.keys_off = keys_off;
                best.vals_off = vals_off;
                return best;
            }
            cap_guess -= 1;
        }

        best
    }

    /// Compute a leaf layout targeting an exact capacity (number of key/value pairs).
    pub fn compute_for_cap<K, V>(cap: u16, doubly_linked: bool) -> Self {
        let a_ptr = align_of::<*const ()>();
        let a_k = align_of::<K>();
        let a_v = align_of::<V>();
        let s_ptr = size_of::<*const ()>();
        let s_k = size_of::<K>();
        let s_v = size_of::<V>();

        let max_align = a_ptr.max(a_k).max(a_v).max(align_of::<NodeHdr>());
        let hdr_size = align_up(size_of::<NodeHdr>(), max_align);

        let sib_bytes = if doubly_linked { 2 * s_ptr } else { s_ptr };
        let sib_off = align_up(hdr_size, a_ptr);
        let after_sib = sib_off + sib_bytes;

        let cap_usize = cap as usize;
        let first_is_keys = a_k >= a_v;
        let (a1, s1, a2, s2) = if first_is_keys {
            (a_k, s_k, a_v, s_v)
        } else {
            (a_v, s_v, a_k, s_k)
        };

        let first_off = align_up(after_sib, a1);
        let second_off = align_up(first_off + cap_usize * s1, a2);
        let end = second_off + cap_usize * s2;
        let end_aligned = align_up(end, max_align);

        let (keys_off, vals_off) = if first_is_keys {
            (first_off, second_off)
        } else {
            (second_off, first_off)
        };

        Self {
            bytes: end_aligned,
            cap,
            max_align,
            hdr_size,
            next_off: sib_off,
            prev_off: if doubly_linked {
                Some(sib_off + s_ptr)
            } else {
                None
            },
            keys_off,
            vals_off,
        }
    }
}

impl BranchLayout {
    /// Compute a branch layout for key type K.
    pub fn compute<K>(bytes: usize) -> Self {
        let a_ptr = align_of::<*const ()>();
        let a_k = align_of::<K>();
        let s_ptr = size_of::<*const ()>();
        let s_k = size_of::<K>();

        let max_align = a_ptr.max(a_k).max(align_of::<NodeHdr>());
        let hdr_size = align_up(size_of::<NodeHdr>(), max_align);

        // quick upper bound ignoring alignment: children (cap+1) pointers + cap keys
        let mut cap_guess = if s_k + s_ptr == 0 {
            0
        } else {
            bytes.saturating_sub(hdr_size) / (s_k + s_ptr)
        };
        if cap_guess > u16::MAX as usize {
            cap_guess = u16::MAX as usize;
        }

        // Children usually have higher alignment; place higher-aligned first.
        let children_first = a_ptr >= a_k;

        // Defaults if nothing fits
        let mut best = Self {
            bytes,
            cap: 0,
            max_align,
            hdr_size,
            children_off: hdr_size,
            keys_off: hdr_size,
        };

        while cap_guess > 0 {
            let first_a = if children_first { a_ptr } else { a_k };
            let first_s = if children_first { s_ptr } else { s_k };
            let first_len = if children_first {
                cap_guess + 1
            } else {
                cap_guess
            };

            let second_a = if children_first { a_k } else { a_ptr };
            let second_s = if children_first { s_k } else { s_ptr };
            let second_len = if children_first {
                cap_guess
            } else {
                cap_guess + 1
            };

            let first_off = align_up(hdr_size, first_a);
            let second_off = align_up(first_off + first_len * first_s, second_a);
            let end = second_off + second_len * second_s;
            let end_aligned = align_up(end, max_align);

            if end_aligned <= bytes {
                best.cap = cap_guess as u16;
                if children_first {
                    best.children_off = first_off;
                    best.keys_off = second_off;
                } else {
                    best.children_off = second_off;
                    best.keys_off = first_off;
                }
                return best;
            }
            cap_guess -= 1;
        }

        best
    }

    /// Compute a branch layout targeting an exact capacity (number of keys).
    pub fn compute_for_cap<K>(cap: u16) -> Self {
        let a_ptr = align_of::<*const ()>();
        let a_k = align_of::<K>();
        let s_ptr = size_of::<*const ()>();
        let s_k = size_of::<K>();
        let max_align = a_ptr.max(a_k).max(align_of::<NodeHdr>());
        let hdr_size = align_up(size_of::<NodeHdr>(), max_align);

        let children_first = a_ptr >= a_k;
        let first_a = if children_first { a_ptr } else { a_k };
        let first_s = if children_first { s_ptr } else { s_k };
        let first_len = if children_first {
            cap as usize + 1
        } else {
            cap as usize
        };

        let second_a = if children_first { a_k } else { a_ptr };
        let second_s = if children_first { s_k } else { s_ptr };
        let second_len = if children_first {
            cap as usize
        } else {
            cap as usize + 1
        };

        let first_off = align_up(hdr_size, first_a);
        let second_off = align_up(first_off + first_len * first_s, second_a);
        let end = second_off + second_len * second_s;
        let end_aligned = align_up(end, max_align);

        let (children_off, keys_off) = if children_first {
            (first_off, second_off)
        } else {
            (second_off, first_off)
        };

        Self {
            bytes: end_aligned,
            cap,
            max_align,
            hdr_size,
            children_off,
            keys_off,
        }
    }
}

// ============================
// Raw carving helpers
// ============================

#[derive(Copy, Clone)]
pub struct LeafParts<K, V> {
    pub hdr: *mut NodeHdr,
    pub next_ptr: *mut *mut u8,
    pub prev_ptr: Option<*mut *mut u8>,
    pub keys_ptr: *mut MaybeUninit<K>,
    pub vals_ptr: *mut MaybeUninit<V>,
}

impl<K, V> LeafParts<K, V> {}

#[derive(Copy, Clone)]
pub struct BranchParts<K> {
    pub hdr: *mut NodeHdr,
    pub children_ptr: *mut MaybeUninit<*mut u8>,
    pub keys_ptr: *mut MaybeUninit<K>,
}

impl<K> BranchParts<K> {}

/// Carve a leaf node's header, sibling pointers, and arrays from a raw base pointer.
#[inline(always)]
pub unsafe fn carve_leaf<K, V>(base: NonNull<u8>, layout: &LeafLayout) -> LeafParts<K, V> {
    let p = base.as_ptr();
    let hdr = p as *mut NodeHdr;
    let next_ptr = p.add(layout.next_off) as *mut *mut u8;
    let prev_ptr = layout.prev_off.map(|off| p.add(off) as *mut *mut u8);
    let keys_ptr = p.add(layout.keys_off) as *mut MaybeUninit<K>;
    let vals_ptr = p.add(layout.vals_off) as *mut MaybeUninit<V>;
    LeafParts {
        hdr,
        next_ptr,
        prev_ptr,
        keys_ptr,
        vals_ptr,
    }
}

/// Carve a branch node's header, children pointers, and keys array from a raw base pointer.
#[inline(always)]
pub unsafe fn carve_branch<K>(base: NonNull<u8>, layout: &BranchLayout) -> BranchParts<K> {
    let p = base.as_ptr();
    let hdr = p as *mut NodeHdr;
    let children_ptr = p.add(layout.children_off) as *mut MaybeUninit<*mut u8>;
    let keys_ptr = p.add(layout.keys_off) as *mut MaybeUninit<K>;
    BranchParts {
        hdr,
        children_ptr,
        keys_ptr,
    }
}
