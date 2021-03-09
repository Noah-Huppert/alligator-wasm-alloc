use cfg_if;

/// The size of one WASM page.
pub const PAGE_BYTES: usize = 65536;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use core::arch::wasm32::{memory_size,memory_grow};

        /// The index specifying which memory wasm should
        /// allocate. Currently in wasm this is only and
        /// always 0.
        const WASM_MEMORY_IDX: u32 = 0;

        /// Implements a heap inside the
        /// WASM environment.
        pub struct HostHeap {}

        impl HostHeap {
            pub fn memory_size(&self) -> usize {
                memory_size(WASM_MEMORY_IDX)
            }

            pub unsafe fn memory_grow(&self, delta_pages: usize) -> usize {
                memory_grow(WASM_MEMORY_IDX, delta_pages)
            }

            pub unsafe fn base_ptr(&self) -> *mut u8 {
                0 as *mut u8
            }
        }

        pub const INIT: HostHeap = HostHeap{};
    } else if #[cfg(unix)] {
        use libc::malloc;

        /// The number of pages allocated to mock
        /// out the WASM heap. Currently 2 GB.
        const MALLOC_PAGES: usize = 31250;
        
        /// Implements a heap using libc malloc.
        pub struct HostHeap {
            /// The host memory region pointer. None if
            /// not allocated.
            host_base_ptr: Option<*mut u8>,

            /// The current end of the guest's memory
            /// in pages.
            guest_end_page: usize,
        }

        impl HostHeap {
            /// Ensure that the host memory has been
            /// allocated. Returns the host_base_ptr value.
            pub unsafe fn ensure_host_base_ptr(&mut self) -> *mut u8 {
                if let Some(ptr) = self.host_base_ptr {
                    return ptr;
                } else {
                    let ptr = malloc(MALLOC_PAGES * PAGE_BYTES) as *mut u8;
                    self.host_base_ptr = Some(ptr);
                    return ptr;
                }
            }
            
            pub fn memory_size(&mut self) -> usize {
                self.guest_end_page
            }

            pub unsafe fn memory_grow(&mut self, delta_pages: usize) -> usize {
                // Lazy allocate the host memory
                self.ensure_host_base_ptr();

                // Ensure not oversize
                let new_guest_end_page = self.guest_end_page + delta_pages;
                if new_guest_end_page > MALLOC_PAGES {
                    // Is over what we can allocate
                    return usize::MAX;
                }

                // Set new guest end page
                let old_guest_page = self.guest_end_page;
                self.guest_end_page = new_guest_end_page;
                
                return old_guest_page;
            }

            pub unsafe fn base_ptr(&mut self) -> *mut u8 {
                // Lazy allocate the host memory,
                // then return base ptr
                self.ensure_host_base_ptr()
            }
        }

        pub const INIT: HostHeap = HostHeap{
            host_base_ptr: None,
            guest_end_page: 0,
        };
    }
}
