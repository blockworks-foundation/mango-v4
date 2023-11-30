#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout};

#[cfg(not(feature = "no-entrypoint"))]
#[global_allocator]
pub static ALLOCATOR: BumpAllocator = BumpAllocator {
    start: solana_program::entrypoint::HEAP_START_ADDRESS as usize,
};

pub fn heap_used() -> usize {
    #[cfg(not(feature = "no-entrypoint"))]
    return ALLOCATOR.used();

    #[cfg(feature = "no-entrypoint")]
    return 0;
}

/// Custom bump allocator for on-chain operations
///
/// The default allocator is also a bump one, but grows from a fixed
/// HEAP_START + 32kb downwards and has no way of making use of extra
/// heap space requested for the transaction.
///
/// This implementation starts at HEAP_START and grows upward, producing
/// a segfault once out of available heap memory.
pub struct BumpAllocator {
    pub start: usize,
}

unsafe impl GlobalAlloc for BumpAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pos_ptr = self.start as *mut usize;

        let mut pos = *pos_ptr;
        if pos == 0 {
            // First time, set starting position
            pos = self.start + 8;
        }

        // Align pos to create the address for the allocation
        let a = layout.align().wrapping_sub(1);
        let mut begin = pos.checked_add(a).unwrap();
        begin &= !a;

        // Update allocator state
        let end = begin.checked_add(layout.size()).unwrap();
        *pos_ptr = end;

        // Write a byte to trigger heap overflow errors early
        let end_ptr = end as *mut u8;
        *end_ptr = 0;

        begin as *mut u8
    }
    #[inline]
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // I'm a bump allocator, I don't free
    }
}

impl BumpAllocator {
    #[inline]
    pub fn used(&self) -> usize {
        unsafe {
            let pos_ptr = self.start as *mut usize;

            let pos = *pos_ptr;
            if pos == 0 {
                return 0;
            }
            return pos - self.start;
        }
    }
}
