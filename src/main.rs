use core::alloc::{GlobalAlloc, Layout};
use core::arch::wasm32;
use core::cell::UnsafeCell;
use core::ptr::{null_mut};

/// The index specifying which memory wasm should
/// allocate. Currently in wasm this is only and
/// always 0.
const WASM_MEMORY_IDX: u32 = 0;

/// The size of one WASM page.
const WASM_PAGE_BYTES: usize = 65536;

/// The number of pages to allocate for all memory
/// allocations. If these fill up then all future
/// allocations will fail.
const ALLOC_PAGES: usize = 65518;

/// The byte address of the last allocatable space of memory
/// in the segment the allocator will work in.
const ALLOC_END_BYTES: u32 = (WASM_PAGE_BYTES as u32) * (ALLOC_PAGES as u32);

/// Allocates an initial number of memory pages, then
/// maintains a pointer to the next free spot in the
/// heap. Very constrained implementation, doesn't ever
/// free memory.
struct AlligatorHeap {
    /// True if the initial call to allocate all the memory
    /// we will use has been made. next_free_ptr is only
    /// guaranteed to not be null when did_init_heap is true.
    did_init_heap: bool,
    
    /// Pointer in the heap to the next free spot.
    next_free_ptr: *mut u8,

    /// The number of bytes currently used within
    /// the heap.
    used_bytes: u32,
}

impl AlligatorHeap {
    pub const INIT: AlligatorHeap = AlligatorHeap{
        did_init_heap: false,
        next_free_ptr: null_mut(),
        used_bytes: 0,
    };
    
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // Check if we are being asked to allocate more
        // than we can.
        let used_bytes_after_alloc = self.used_bytes + (layout.size() as u32);
        if used_bytes_after_alloc > ALLOC_END_BYTES {
            // We are being asked to allocate memory we
            // don't have.
            return null_mut();
        }

        // Don't allow 0 sized allocations
        if layout.size() == 0 {
            return null_mut();
        }

        // Check if the allocator has grabbed its pages
        // from the host yet.
        if self.did_init_heap == false {
            // If the pages haven't been grabbed yet
            // Determine delta pages we need to grow by
            let current_pages = wasm32::memory_size(WASM_MEMORY_IDX);
            let delta_pages = ALLOC_PAGES - current_pages;
            
            // Request the memory is grown via the host.
            // grow_res will be the number of pages
            // before the grow, and thus the start
            // our new allocated pages, or usize::MAX
            // if error.  
            let grow_res = wasm32::memory_grow(WASM_MEMORY_IDX, delta_pages);
            if grow_res == usize::MAX {
                // Failed to allocate the memory we need
                // from the host
                return null_mut();
            }

            // Set next free byte to the start of our region
            self.next_free_ptr = (grow_res * WASM_PAGE_BYTES) as *mut u8;
            self.did_init_heap = true;
        }

        // Construct a pointer to the memory segment
        // we will allocate
        let alloc_at = self.next_free_ptr;
        // while alloc_at % (layout.align() as u8) != 0 {
        //     alloc_at += 1;
        // }

        // Update the next free byte pointer to after
        // the segment of memory we are abou to allocate
        self.next_free_ptr = self.next_free_ptr.offset(layout.size() as isize);
        self.used_bytes += layout.size() as u32;

        return alloc_at;
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        // Currently nothing can be freed. The
        // next_free_ptr pointer is only incremented.
    }
}

/// The custom global allocator. Wraps the AlligatorHeap
/// which performs all memory operations. See for more details.
struct AlligatorAlloc {
    /// Data structure which keeps state of all memory
    /// wrapped inside an UnsafeCell for
    /// memory symantics.
    heap: UnsafeCell<AlligatorHeap>,
}

unsafe impl Sync for AlligatorAlloc {}

impl AlligatorAlloc {
    pub const INIT: AlligatorAlloc = AlligatorAlloc{
        heap: UnsafeCell::new(AlligatorHeap::INIT),
    };
}

unsafe impl GlobalAlloc for AlligatorAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        //return 0 as *mut u8;
        return (*self.heap.get()).alloc(layout);
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        (*self.heap.get()).dealloc(ptr, layout);
    }
}

/*
 * What follows is the benchmark program. Right now it
 * just tries to get any sort of memory allocation
 * to occur.
 */
#[global_allocator]
static ALLOC: AlligatorAlloc = AlligatorAlloc::INIT;

fn main() {
    for i in 0..100000 {
        greet(&format!("Alligator wasmtime, i={}", i));
    }
}

fn greet(name: &str) {
    println!("hello {}", name);
}
