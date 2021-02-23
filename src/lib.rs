use core::alloc::{GlobalAlloc, Layout};
use core::arch::wasm32;
use core::cell::UnsafeCell;
use core::ptr::{null_mut};

use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use std::panic;
//use web_sys::console;

//extern crate wee_alloc;

/// The index specifying which memory wasm should
/// allocate. Currently in wasm this is only and
/// always 0.
const WASM_MEMORY_IDX: u32 = 0;

/// The size of one WASM page.
const WASM_PAGE_BYTES: usize = 65536;

/// The maximum number of pages ever allocated. A hard
/// upper limit was defined so page information could be
/// kept track of on the stack in a fixed sized array.
const ALLOC_MAX_PAGES: usize = 100;

struct AlligatorHeap {
    /// Keeps track of each page's allocation status. If
    /// a value at an index is true that means the page
    /// of memory at that index is free. False means the
    /// page of memory is allocated.
    free_status: [bool; ALLOC_MAX_PAGES],
}

impl AlligatorHeap {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // Check if we are being asked to allocate more
        // than one page.
        if layout.size() > WASM_PAGE_BYTES {
            // We are being asked to allocate memory
            // larger than a page.
            return null_mut();
        }

        // Don't allow 0 sized allocations
        if layout.size() == 0 {
            return null_mut();
        }
        
        // Get current number of allocated pages
        let num_pages = wasm32::memory_size(WASM_MEMORY_IDX);

        // // Find a free page
        // for i in 0..num_pages {
        //     if self.free_status[i] {
        //         // Page is free, mark as allocated
        //         self.free_status[i] = false;

        //         let page_ptr = (WASM_PAGE_BYTES * i) as *mut u8;
        //         return page_ptr;
        //     }
        // }

        // // Check if at max pages
        // if num_pages == ALLOC_MAX_PAGES {
        //     // At maximum number of pages
        //     return null_mut();
        // }

        // Allocate a new page
        let grow_res = wasm32::memory_grow(WASM_MEMORY_IDX, 1);
        if grow_res == usize::MAX {
            // Failed to grow
            return null_mut();
        }

        //let page_ptr = (WASM_PAGE_BYTES * (num_pages+1)) as *mut u8;
        let page_ptr = (WASM_PAGE_BYTES * (num_pages)) as *mut u8;
        return page_ptr;
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        let page_idx = (((*ptr) as f32) / (WASM_PAGE_BYTES as f32)).floor() as usize;
        self.free_status[page_idx] = true;
    }
}

/// The custom allicator. Currently a very constrained
/// implementation. It can only allocate memory up to the
/// size of WASM_PAGE_BYTES, no larger. Each allocation
/// gets its own page. If more than ALLOC_MAX_PAGES pages
/// are allocated allocation will fail.
struct AlligatorAlloc {
    /// Data structure which keeps state of all memory
    /// wrapped inside an UnsafeCell for
    /// memory symantics.
    heap: UnsafeCell<AlligatorHeap>,
}

unsafe impl Sync for AlligatorAlloc {}

impl AlligatorAlloc {
    pub const INIT: AlligatorAlloc = AlligatorAlloc{
        heap: UnsafeCell::new(AlligatorHeap{
            free_status: [true; ALLOC_MAX_PAGES],
        }),
    };
}

unsafe impl GlobalAlloc for AlligatorAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        //return 0 as *mut u8;
        return (*self.heap.get()).alloc(layout);
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        //(*self.heap.get()).dealloc(ptr, layout);
    }
}

/*
 * What follows is the benchmark program. Right now it
 * just tries to get any sort of memory allocation
 * to occur.
 */
#[global_allocator]
static ALLOC: AlligatorAlloc = AlligatorAlloc::INIT;

// #[wasm_bindgen(start)]
// pub fn main() {
//     panic::set_hook(Box::new(console_error_panic_hook::hook));
// }

#[wasm_bindgen]
extern {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("hello {}", name));
}
