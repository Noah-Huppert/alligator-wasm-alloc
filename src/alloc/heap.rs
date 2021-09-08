use cfg_if::cfg_if;
use core::ptr::null_mut;

/// The size of one WASM page.
pub const PAGE_BYTES: u32 = 65536;

/// PAGE_BYTES as isize
pub const PAGE_BYTES_ISIZE: isize = PAGE_BYTES as isize;

/// The maximum number of pages which can be allocated. Defined by the WebAssembly spec: https://webassembly.github.io/spec/js-api/index.html#limits
/// Specification says 65536, we do one less to fit into u32.
pub const MAX_PAGES: u32 = 65535;

/// Host heap implementation. How the memory actually gets allocated by the operating system / runtime. Acts as one contiguous memory segment.
/// Emulates the WASM memory model.
pub trait HostHeap {
    /// Return the heap size in pages.
    fn memory_size(&mut self) -> usize;

    /// Grow the heap by a number of pages.
    /// Returns the heap size in pages
    /// before the grow if successful, or usize::MAX
    /// if error.
    unsafe fn memory_grow(&mut self, delta_pages: usize) -> usize;

    /// Returns the base address of the specified
    /// Addresses will be guaranteed contiguous
    /// for the following memory_size() bytes.
    unsafe fn base_ptr(&mut self) -> *mut u8;
}

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use core::arch::wasm32::{memory_size,memory_grow};

        /// The index specifying which memory wasm should
        /// allocate. Currently in wasm this is only and
        /// always 0.
        const WASM_MEMORY_IDX: u32 = 0;

        /// Implements a heap inside the WASM environment.
        pub struct WASMHostHeap {
            /// Stores the base memory address of the heap space which the allocator will manage. See ensure_base_found() for more details.
            mem_base_ptr: Option<*mut u8>,
        }

        impl WASMHostHeap {
            /// Returns the base address of the heap space which this allocator will manage. If not set it will determine the base address, store it for the future, and return it. This will always be the actual base heap pointer because this method is called in memory_grow() before the memory is actually grown.
            unsafe fn ensure_base_found(&mut self) -> *mut u8 {
                match self.mem_base_ptr {
                    Some(ptr) => ptr, // Base pointer already known
                    None => {
                        // Base pointer is not known: means the heap has not been grown by WASMHostHeap. Therefore all existing memory is not ours to manage, and we should consider the base of our heap as right after this existing memory.
                        let init_size = self.memory_size();

                        let ptr = (0 as *mut u8).offset((init_size as isize) * PAGE_BYTES_ISIZE);
                        
                        self.mem_base_ptr = Some(ptr);

                        ptr
                    },
                }
            }
        }

        impl HostHeap for WASMHostHeap {
            /// Returns the size of the current heap in pages.
            fn memory_size(&mut self) -> usize {
                memory_size(WASM_MEMORY_IDX)
            }

            /// Grows the heap by a number of pages.
            unsafe fn memory_grow(&mut self, delta_pages: usize) -> usize {
                self.ensure_base_found();
                memory_grow(WASM_MEMORY_IDX, delta_pages)
            }

            /// Returns a pointer to the beginning of the allocators heap segment.
            unsafe fn base_ptr(&mut self) -> *mut u8 {
                self.ensure_base_found()
            }
        }

        /// Pre-initialized WASM HostHeap.
        pub const INIT: WASMHostHeap = WASMHostHeap{
            mem_base_ptr: None,
        };

        pub type HeapType = WASMHostHeap;
    } else if #[cfg(all(unix, target_pointer_width = "32"))] {
        use libc::malloc;

	   /// The number of pages which can actually be used. This number is currently limited because malloc calls for the full 4 GB don't succeed in Rust (but I can get them to work in a C program). So for now just limit size of LibC HostHeap implementation.
	   const ACTUAL_EMULATED_PAGES: u32 = 10;

        /// Implements a heap using libc malloc.
	   ///
	   /// Limited
        pub struct LibCHostHeap {
            /// The host memory region pointer. None if not allocated.
            host_base_ptr: Option<*mut u8>,

            /// The current end of the guest's memory in pages.
            guest_end_page: usize,
        }

        impl LibCHostHeap {
            /// Ensure that the host memory has been allocated. Returns the host_base_ptr value.
            unsafe fn ensure_host_base_ptr(&mut self) -> Result<*mut u8, ()> {
                match self.host_base_ptr {
				Some(ptr) => Ok(ptr),
				None => {
                        let ptr = malloc((ACTUAL_EMULATED_PAGES * PAGE_BYTES) as usize) as *mut u8;
				    if ptr.is_null() {
					   // Failed to malloc
					   return Err(());
				    }
				    
                        self.host_base_ptr = Some(ptr);
                        Ok(ptr)
				},
                }
            }
        }

        impl HostHeap for LibCHostHeap {
            /// Returns the heap's size in pages.
            fn memory_size(&mut self) -> usize {
                self.guest_end_page
            }

            /// Grows the heap by a number of pages.
            unsafe fn memory_grow(&mut self, delta_pages: usize) -> usize {
                // Lazy allocate the host memory
                match self.ensure_host_base_ptr() {
				Err(_) => return usize::MAX, // failure
				_ => {},
			 };

                // Ensure not oversize
                let new_guest_end_page = self.guest_end_page + delta_pages;
                if new_guest_end_page > MAX_PAGES as usize || new_guest_end_page > ACTUAL_EMULATED_PAGES as usize {
                    // Is over what we can allocate
                    return usize::MAX;
                }

                // Set new guest end page
                let old_guest_page = self.guest_end_page;
                self.guest_end_page = new_guest_end_page;
                
                return old_guest_page;
            }

            /// Returns a pointer to the base of the heap segment the allocator will manage.
            unsafe fn base_ptr(&mut self) -> *mut u8 {
                // Lazy allocate the host memory, then return base ptr
                match self.ensure_host_base_ptr() {
				Ok(ptr) => ptr,
				Err(_) => null_mut(),
			 }
            }
        }

        /// Pre-initialized 32-bit LibC HostHeap.
        pub const INIT: LibCHostHeap = LibCHostHeap{
            host_base_ptr: None,
            guest_end_page: 0,
        };

        pub type HeapType = LibCHostHeap;
    }
}
