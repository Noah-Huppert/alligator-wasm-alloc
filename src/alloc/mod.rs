use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;
use std::mem::size_of;

mod heap;

/// The number of pages to allocate for all memory
/// allocations. If these fill up then all future
/// allocations will fail.
const ALLOC_PAGES: usize = 100;

/// The byte address of the last allocatable space of memory
/// in the segment the allocator will work in.
const ALLOC_END_BYTES: u32 = (heap::PAGE_BYTES as u32) * (ALLOC_PAGES as u32);

/// Allocates an initial number of memory pages, then
/// maintains a free linked list.
struct AllocatorImpl {
    /// True if the initial call to allocate all the memory
    /// we will use has been made. free_list_head is only
    /// guaranteed to not be null when did_init_heap is true.
    did_init_heap: bool,
    
    /// The HostHeap implementation for the
    /// current platform.
    heap: UnsafeCell<heap::HostHeap>,

    /// The head of the free list. 
    free_list_head: *mut HeapFreeNode,
}

/// A free list node in the heap.
struct HeapFreeNode {
    /// The next free node. None if this is the
    /// last node.
    next: Option<*mut HeapFreeNode>,

    /// Size of the segment in bytes. Not
    /// including this header.
    size_bytes: u32,

    /// If free.
    free: bool,
}

impl AllocatorImpl {
    pub const INIT: AllocatorImpl = AllocatorImpl{
        did_init_heap: false,
        heap: UnsafeCell::new(heap::INIT),
        free_list_head: null_mut(),
    };
    
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // Size of actual allocation including free list header
        let alloc_bytes = layout.size() + size_of::<HeapFreeNode>();

        // Don't allow 0 sized allocations
        if layout.size() == 0 {
            return null_mut();
        }

        // Check if the allocator has grabbed its pages
        // from the host yet.
        if !self.did_init_heap {
            // If the pages haven't been grabbed yet
            // Determine delta pages we need to grow by
            let current_pages = (*self.heap.get()).memory_size();
            let delta_pages = ALLOC_PAGES - current_pages;
            
            // Request the memory is grown via the host.
            // grow_res will be the number of pages
            // before the grow, and thus the start
            // our new allocated pages, or usize::MAX
            // if error.  
            let grow_res = (*self.heap.get()).memory_grow(delta_pages);
            if grow_res == usize::MAX {
                // Failed to allocate the memory we need
                // from the host
                return null_mut();
            }

            // Create a free list encompassing our allocated region
            let base_ptr = (*self.heap.get()).base_ptr().offset((grow_res * heap::PAGE_BYTES) as isize);
            
            self.free_list_head = base_ptr as *mut HeapFreeNode;
            (*self.free_list_head).next = None;
            (*self.free_list_head).size_bytes = ((delta_pages * heap::PAGE_BYTES) - size_of::<HeapFreeNode>()) as u32;
            (*self.free_list_head).free = true;
            
            self.did_init_heap = true;
        }

        // Try to find a free segment for allocation
        let mut head = Some(self.free_list_head);
        while let Some(node) = head {
            if !(*node).free || (*node).size_bytes < (alloc_bytes as u32) {
                // Current node won't work, look at next one
                head = (*node).next;
            } else {
                // This node will work, allocate!
                (*node).free = false;
                let alloc_ptr = node.offset(1) as *mut u8;

                // Check if we can split this region or if
                // the allocation will use it all
                let extra_bytes = (*node).size_bytes - ((layout.size() + size_of::<HeapFreeNode>()) as u32);
                if extra_bytes > 0 {
                    // The extra bytes can hold at least the
                    // free node header plus some, so it is
                    // safe to split up this segment.
                    let mut split_header = alloc_ptr.offset(layout.size() as isize) as *mut HeapFreeNode;
                    (*split_header).next = (*node).next;
                    (*split_header).size_bytes = extra_bytes;
                    (*split_header).free = true;

                    (*node).next = Some(split_header);
                    (*node).size_bytes = layout.size() as u32;
                }

                return alloc_ptr;
            }
        }

        // If we didn't find a free segment, there's no
        // possible space in our heap suitable
        return null_mut();

        // TODO: Make align correctly

        // // Construct a pointer to the memory segment
        // // we will allocate
        // let alloc_at = self.next_free_ptr;
        // // let alloc_at = self.next_free_ptr + (self.next_free_ptr % (layout.align() as u8))
        // // while alloc_at % (layout.align() as u8) != 0 {
        // //     alloc_at += 1;
        // // }

        // // Update the next free byte pointer to after
        // // the segment of memory we are abou to allocate
        // self.next_free_ptr = self.next_free_ptr.offset(layout.size() as isize);
        // self.used_bytes += layout.size() as u32;

        // return alloc_at;
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        let mut node = (ptr as *mut HeapFreeNode).offset(-1);
        (*node).free = true;
    }
}

/// The custom global allocator. Wraps the AllocatorImpl
/// which performs all memory operations. See for more details.
pub struct AlligatorAlloc {
    /// Data structure which keeps state of all memory
    /// wrapped inside an UnsafeCell for
    /// memory symantics.
    heap: UnsafeCell<AllocatorImpl>,
}

unsafe impl Sync for AlligatorAlloc {}

impl AlligatorAlloc {
    pub const INIT: AlligatorAlloc = AlligatorAlloc{
        heap: UnsafeCell::new(AllocatorImpl::INIT),
    };
}

unsafe impl GlobalAlloc for AlligatorAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        return (*self.heap.get()).alloc(layout);
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        (*self.heap.get()).dealloc(ptr, layout);
    }
}
