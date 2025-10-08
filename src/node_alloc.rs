extern crate alloc;

use alloc::alloc::{alloc, dealloc, Layout};
use core::ptr::{self, NonNull};

use crate::layout::{carve_leaf, BranchLayout, LeafLayout, NodeHdr, NodeTag};

#[inline]
fn layout_for(bytes: usize, align: usize) -> Layout {
    // SAFETY: align is computed from type/layout alignments => power of two, non-zero.
    Layout::from_size_align(bytes, align).expect("invalid layout")
}

#[inline]
pub unsafe fn alloc_raw(bytes: usize, align: usize) -> Option<NonNull<u8>> {
    let layout = layout_for(bytes, align);
    let p = alloc(layout);
    NonNull::new(p)
}

#[inline]
pub unsafe fn dealloc_raw(ptr: NonNull<u8>, bytes: usize, align: usize) {
    let layout = layout_for(bytes, align);
    dealloc(ptr.as_ptr(), layout);
}

/// Allocate a leaf node block and initialize its header and sibling pointers.
#[inline]
pub unsafe fn alloc_leaf_block(layout: &LeafLayout) -> Option<NonNull<u8>> {
    let p = alloc_raw(layout.bytes, layout.max_align)?;
    init_leaf_block(p, layout);
    Some(p)
}

/// Initialize an existing leaf block's header and siblings to defaults.
#[inline]
pub unsafe fn init_leaf_block(base: NonNull<u8>, layout: &LeafLayout) {
    // Header
    let hdr = base.as_ptr() as *mut NodeHdr;
    ptr::write(
        hdr,
        NodeHdr {
            tag: NodeTag::Leaf,
            len: 0,
            flags: 0,
        },
    );

    // Sibling pointers set to null by default
    let parts = carve_leaf::<(), ()>(base, layout);
    ptr::write(parts.next_ptr, core::ptr::null_mut());
    if let Some(prev) = parts.prev_ptr {
        ptr::write(prev, core::ptr::null_mut());
    }
}

/// Allocate a branch node block and initialize its header.
#[inline]
pub unsafe fn alloc_branch_block(layout: &BranchLayout) -> Option<NonNull<u8>> {
    let p = alloc_raw(layout.bytes, layout.max_align)?;
    init_branch_block(p);
    Some(p)
}

/// Initialize an existing branch block's header to defaults.
#[inline]
pub unsafe fn init_branch_block(base: NonNull<u8>) {
    let hdr = base.as_ptr() as *mut NodeHdr;
    ptr::write(
        hdr,
        NodeHdr {
            tag: NodeTag::Branch,
            len: 0,
            flags: 0,
        },
    );
}
