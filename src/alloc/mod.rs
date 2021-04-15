use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;
use std::mem::{size_of,transmute};
use std::convert::TryFrom;

pub mod heap;

use heap::{HostHeap,HeapType};

/// The number of host memory pages to allocate for all memory allocations. If these fill up then all future allocations will fail.
const MAX_HOST_PAGES: usize = 100;

/// MAX_HOST_PAGES as an isize.
const MAX_HOST_PAGES_ISIZE: isize = MAX_HOST_PAGES as isize;

/// Number of bytes which can be allocated from one MiniPage.
const MINI_PAGE_ALLOCABLE_BYTES: u16 = 2048;

/// Size of a mini page with the header.
const MINI_PAGE_TOTAL_BYTES: isize = (MINI_PAGE_ALLOCABLE_BYTES as isize) + (size_of::<MiniPageHeader>() as isize);
/// MINI_PAGE_TOTAL_BYTES as an usize
const MINI_PAGE_TOTAL_BYTES_USIZE: usize = MINI_PAGE_TOTAL_BYTES as usize;
/// MINI_PAGE_TOTAL_BYTES as an i32
const MINI_PAGE_TOTAL_BYTES_I32: i32 = MINI_PAGE_TOTAL_BYTES as i32;
/// MINI_PAGE_TOTAL_BYTES as an u32
const MINI_PAGE_TOTAL_BYTES_U32: u32 = MINI_PAGE_TOTAL_BYTES as u32;
/// MINI_PAGE_TOTAL_BYTES as an f64
const MINI_PAGE_TOTAL_BYTES_F64: f64 = MINI_PAGE_TOTAL_BYTES as f64;


/// The largest size class we can allocate right now.
/// Multi-page allocations are not supported yet.
const MAX_SIZE_CLASS: u8 = 11;

/// MAX_SIZE_CLASS but a usize
const MAX_SIZE_CLASS_USIZE: usize = 11;

/// Maximum number of items stored in a FreeStack.
const FREE_STACK_MAX: usize = 256;

/// Allocates an initial number of memory pages, then
/// maintains a free linked list.
struct AllocatorImpl<H> where H: HostHeap {
    /// True if the initial call to allocate all the
    /// memory we will use has been made.
    /// next_minipage_addr is only
    /// guaranteed to not be null when did_init_heap
    /// is true.
    did_init_heap: bool,
    
    /// The HostHeap implementation for the
    /// current platform.
    heap: UnsafeCell<H>,

    /// Head of MiniPage header free list for each size class.
    free_lists: [*mut MiniPageHeader; MAX_SIZE_CLASS_USIZE],

    /// The most recently used free_lists node for each MiniPage.
    free_list_currents: [*mut MiniPageHeader; MAX_SIZE_CLASS_USIZE],

    /// Next address which can be used for a MiniPage.
    next_minipage_addr: *mut u8,

    /// Free segment indexes for the head of each size class. This is an array of circular stacks. Metadata for these circular stacks is stored in free_segments_{sizes,head_idxs}. The free_segments_{push,pop} methods mutate these stacks.
    free_segments: [[u16; FREE_STACK_MAX]; MAX_SIZE_CLASS_USIZE],

    /// Size of each stack in free_segments.
    free_segments_sizes: [usize; MAX_SIZE_CLASS_USIZE],

    /// Head index of each stack in free_segments.
    free_segments_head_idxs: [usize; MAX_SIZE_CLASS_USIZE],
}

/// Header for a MiniPage.
struct MiniPageHeader {
    /// The next free node of the same size class.
    next: Option<*mut MiniPageHeader>,

    /// Size class exponent
    size_class_exp: u8,

    /// Bit-packed free list. A 1 means that segment is free, 0 means allocated.
    free_segments: [u8; 256],
}

/// Calculates useful size class values.
#[derive(Copy, Clone)]
struct SizeClass {
    /// Exponent value n in 2^n which identifies size class.
    exp: u8,
}

impl SizeClass {
    /// New size class from an exponent number.
    fn new(exp: u8) -> SizeClass {
        SizeClass{
            exp: exp,
        }
    }

    /// Creates the size class required to fit a number of bytes.
    fn new_from_bytes(bytes: u16) -> SizeClass {
        let fbytes = f32::from(bytes);
        let exp = fbytes.log2().ceil() as u8;

        SizeClass{
            exp: exp,
        }
    }

    /// Exponent as a usize, useful for indexing into arrays.
    fn exp_usize(self) -> usize {
        usize::from(self.exp)
    }
    
    /// Size of a segment in bytes.
    /// Returns u16 because the maximum size class is 11 and 2^11 requires at least 11 bits, thus 16 are required.
    fn segment_bytes(self) -> u16 {
        2_u16.pow(u32::from(self.exp))
    }

    /// Returns the maximum number of segments which can be stored in a MiniPage for this size class.
    fn segments_max_num(&self) -> u16 {
        MINI_PAGE_ALLOCABLE_BYTES / self.segment_bytes()
    }
}

/// Represents an address in memory within the context of the memory allocator design.
/// TODO: Convert AllocatorImpl to use MemAddr.
#[derive(Copy, Clone)]
struct MemAddr {
    /// Numerical representation of address. This is used to complete all pointer math against.
    /// The unsafe as_ptr() method will turn this number into a memory address which is gaurenteed to be safe.
    /// This address assumes the base memory address of the heap is 0. This simplifies calculations. The actual heap base offset is added via as_ptr().
    addr: u32,
}

impl MemAddr {
    /// Initializes a MemAddr from a u32.
    fn new(addr: u32) -> MemAddr {
        MemAddr{
            addr: addr,
        }
    }

    /// Initializes a MemAddr from a usize.
    ///
    /// # Panics
    /// Shouldn't panic because:
    /// - program only supports 32 bit machines => usize will be 32 bits
    /// - usize is 32 bits => cast from usize to u32 shouldn't fail
    fn from_usize(u: usize) -> MemAddr {
        MemAddr{
            addr: u as u32,
        }
    }

    /// Initializes a MemAddr from a raw pointer, a heap base pointer, and a SizeClass.
    /// 
    /// # Safety
    /// Calls std::mem::transmute which is only safe if the result type is the same size as the input type. For this method this is the case because:
    /// - program only supports 32 bit addresses => a raw pointer will be a 32 bit unsigned number
    /// - target type of transmute is a 32 bit unsigned integer => transmute call is safe
    unsafe fn from_ptr(base_ptr: *mut u8, ptr: *mut u8) -> MemAddr {
        MemAddr{
            addr: transmute::<*mut u8, u32>(ptr) - transmute::<*mut u8, u32>(base_ptr),
        }
    }

    /// Returns information about the MiniPage from which this memory address was allocated.
    fn get_page_meta(self) -> MiniPageMeta {
        MiniPageMeta::from_addr(self)
    }

    /// Returns information about the segment from which this memory address was allocated.
    fn get_segment(self, size_class: SizeClass) -> MiniPageSegment {
        MiniPageSegment::from_addr(size_class, self)
    }

    /// Returns addr as an unsafe pointer gaurenteed not to be null.
    /// 
    /// # Safety
    /// The returned pointer will is known to be valid because:
    /// - It will be no earlier than the base pointer of the host heap => Pointer is not refering to memory too early in the heap, thus memory we may not control or does not exist.
    ///
    /// However the .addr field is not checked for correctness => The resulting pointer will only be safe if addr is not larger than the total host heap.
    unsafe fn as_ptr(self, base_ptr: *mut u8) -> *mut u8 {
        // # Panics
        // Should not panic because:
        // - program only supports 32 bit memory addresses => isize will be 32 bits
        // - .addr should always refer to a valid 32 bit address (up to user of MemAddr to ensure) => .addr + base_ptr will always fit in 32 bits
        // - isize is 32 bits and resulting memory address will always fit in 32 bits => cast to isize will not fail
        base_ptr.offset(isize::try_from(self.addr).unwrap())
    }

    /// Returns the .addr field as a f64.
    fn addr_f64(self) -> f64 {
        f64::from(self.addr)
    }

    /// Returns the .addr field as a usize.
    /// # Panics
    /// usize::try_from should always work since this program only supports 32-bit addresses (aka usize will be 32 bits) and addr is u32 (32 bits).
    fn addr_usize(self) -> usize {
        usize::try_from(self.addr).unwrap()
    }
}

/// Holds metadata about a MiniPage which can be used for calculations.
/// This is different from MiniPageHeader which is a data structure which will be stored directly in the heap.
#[derive(Copy, Clone)]
struct MiniPageMeta {
    /// The index of the MiniPage within the heap. 
    page_idx: usize,

    /// The address in memory at which the page's header starts.
    header_addr: MemAddr,

    /// The address in memory at which the page's segments start.
    segments_start_addr: MemAddr,
}

impl MiniPageMeta {
    /// Determines MiniPageMeta information from a MemAddr.
    fn from_addr(addr: MemAddr) -> MiniPageMeta {
        // # Panics
        // Shouldn't panic because:
        // - program only supported with 32-bit addresses (so usize will be 32 bits) so 32 bit data will fit.
        // - dividing two f64's which are only holding u32 values => cast back to 32 bit data shouldn't overflow
        // - division result is floored so result should be an integer (this also forces any address within the page to map to the correct page)
        let page_idx: usize = usize::try_from((addr.addr_f64() / MINI_PAGE_TOTAL_BYTES_F64).floor() as u32).unwrap();

        // Determine the segment within the page
        let page_header_addr: usize = MINI_PAGE_TOTAL_BYTES_USIZE * page_idx;
        let page_segments_start_addr: usize = page_header_addr + size_of::<MiniPageHeader>();

        MiniPageMeta{
            page_idx: page_idx,
            header_addr: MemAddr::from_usize(page_header_addr),
            segments_start_addr: MemAddr::from_usize(page_segments_start_addr),
        }
    }

    /// Returns the page_idx as a u32.
    fn page_idx_u32(self) -> u32 {
        self.page_idx as u32
    }

    /// Returns a pointer to the MiniPage's header in the heap.
    ///
    /// # Safety
    /// TODO
    unsafe fn get_header_ptr(self, base_ptr: *mut u8) -> *mut MiniPageHeader {
        let header_addr = MemAddr::new(self.page_idx_u32() * MINI_PAGE_TOTAL_BYTES_U32);

        header_addr.as_ptr(base_ptr) as *mut MiniPageHeader
    }

    /// Returns a MiniPageSegment refering to segment_idx of size_class.
    fn get_segment(self, size_class: SizeClass, segment_idx: usize) -> MiniPageSegment {
        MiniPageSegment::from_addr(size_class, MemAddr::from_usize(
            self.segments_start_addr.addr_usize() + (usize::from(size_class.segment_bytes()) * segment_idx)
        ))
    }
}

/// Holds information about a segment in a MiniPage.
#[derive(Copy, Clone)]
struct MiniPageSegment {
    /// Page within which segment resides.
    page: MiniPageMeta,

    /// Size class of the segment.
    size_class: SizeClass,
    
    /// The segment index within a MiniPage
    segment_idx: usize,

    /// The byte within a bit-map in which the bit for this MiniPage Segment is located.
    bitmap_byte_idx: usize,

    /// The bit within the byte refered to by bitmap_byte_idx which refers to this MiniPageSegment. Range [0, 7].
    bitmap_byte_bit_idx: usize,
}

impl MiniPageSegment {
    /// Creates a MiniPageSegment from a MemAddr.
    fn from_addr(size_class: SizeClass, addr: MemAddr) -> MiniPageSegment {
        // Determine the MiniPage
        let page = MiniPageMeta::from_addr(addr);

        let segment_relative_addr: usize = addr.addr_usize() - page.segments_start_addr.addr_usize();
        // # Panics
        // Shouldn't panic because:
        // - program only supports 32 bit addresses => usize will be 32 bits
        // - floor() called on result => result number will be integer
        // - converted f64s represent 32 bit data => division of the two should be 32 bits if integer
        let segment_idx_u32: u32 = ((f64::from(segment_relative_addr as u32) / f64::from(size_class.segment_bytes() as u32)).floor()) as u32;
        // # Panics
        // Shouldn't panic because:
        // - program only supports 32 bit addresses => usize will be 32 bits
        // - cast from 32 bit unsigned integer to usize should not fail => usize::try_from always = Result not Err.
        let segment_idx: usize = usize::try_from(segment_idx_u32).unwrap();

        // Determine the bitmap byte index
        // # Panics
        // Shouldn't panic because:
        // - program only supports 32 bit addresses => usize will be 32 bits
        // - dividing a u32 => usize cast to u32 shouldn't fail
        let bitmap_byte_idx: usize = usize::try_from((f64::from(segment_idx_u32) / 8.0).floor() as u32).unwrap();
        let bitmap_byte_bit_idx: usize = segment_idx % 8;
        
        MiniPageSegment{
            page: page,
            size_class: size_class,
            segment_idx: segment_idx,
            bitmap_byte_idx: bitmap_byte_idx,
            bitmap_byte_bit_idx: bitmap_byte_bit_idx,
        }
    }

    /// Creates a new MiniPageSegment which represents the next segment after the current one.
    /// Returns None if this MiniPageSegment is the last segment and none follow.
    fn next_segment(self) -> Option<MiniPageSegment> {
        // Get next segment memory address
        let seg = MiniPageSegment::from_addr(self.size_class, MemAddr::new(
            self.as_addr().addr + u32::from(self.size_class.segment_bytes())
        ));

        // Check not overflowing the MiniPage
        if seg.segment_idx > usize::from(self.size_class.segments_max_num()) - 1 {
            return None;
        }

        Some(seg)
    }

    /// Returns the start of the segment as memory address.
    fn as_addr(self) -> MemAddr {
        let seg_start_addr = self.page.segments_start_addr.addr_usize();
        let seg_offset = self.segment_idx * usize::from(self.size_class.segment_bytes());
        
        MemAddr::from_usize(seg_start_addr + seg_offset)
    }

    /// Write to a MiniPage's header free bitmap. Free: true = free, false = not-free.
    ///
    /// # Safety
    /// TODO
    unsafe fn write_free_bitmap(self, base_ptr: *mut u8, free: bool) {
        // Get MiniPage header
        let minipage_header = self.page.get_header_ptr(base_ptr);

        // Determine what to write to the header
        let write_bit: u8 = match free {
            true => 1, // un-allocated
            false => 0, // allocated
        };

        // Write
        let free_segments_byte_ptr = &mut (*minipage_header).free_segments[self.bitmap_byte_idx];
        *free_segments_byte_ptr = (write_bit << self.bitmap_byte_bit_idx) | *free_segments_byte_ptr;
    }

    /// Returns the segment's free status from its MiniPage header free bitmap. Returns true if free and false if not-free.
    ///
    /// Safety:
    /// TODO
    unsafe fn get_free_bitmap(self, base_ptr: *mut u8) -> bool {
        // Get the MiniPage header
        let minipage_header = self.page.get_header_ptr(base_ptr);
        
        let search_mask = 1 << self.bitmap_byte_bit_idx;
        let bit_free_status = ((*minipage_header).free_segments[self.bitmap_byte_idx] & search_mask) >> self.bitmap_byte_bit_idx;

        match bit_free_status {
            1 => true,
            _ => false,
        }
    }
}

impl AllocatorImpl<HeapType> {
    /// Initialized allocator structure with a WASMHostHeap.
    pub const INIT: AllocatorImpl<HeapType> = AllocatorImpl{
        did_init_heap: false,
        heap: UnsafeCell::new(heap::INIT),
        free_lists: [null_mut(); MAX_SIZE_CLASS_USIZE],
        free_list_currents: [null_mut(); MAX_SIZE_CLASS_USIZE],
        next_minipage_addr: null_mut(),
        free_segments: [[0; FREE_STACK_MAX]; MAX_SIZE_CLASS_USIZE],
        free_segments_sizes: [0; MAX_SIZE_CLASS_USIZE],
        free_segments_head_idxs: [0; MAX_SIZE_CLASS_USIZE],
    };
}

impl<H> AllocatorImpl<H> where H: HostHeap {
    /// Push an item onto a free_segments stack. Returns Some(()) if succeeded and None if the stack is full.
    fn free_segments_push(&mut self, size_class_exp: u8, item: u16) -> Option<()> {
        // Get metadata about this size class's stack
        let fs_idx = usize::from(size_class_exp);
        
        let size_ptr = &mut self.free_segments_sizes[fs_idx];
        let head_idx_ptr = &mut self.free_segments_head_idxs[fs_idx];
        let data = &mut self.free_segments[fs_idx];
        
        
        // Check if stack is full
        if *size_ptr + 1 > FREE_STACK_MAX {
            return None;
        }

        // Push to data
        let next_data_idx = (*head_idx_ptr + * size_ptr) % FREE_STACK_MAX;
        data[next_data_idx] = item;
        *size_ptr += 1;

        Some(())
    }

    /// Pops an item from the head of a free_segments stack. Returns Some if succeeded and None if the stack is empty.
    fn free_segments_pop(&mut self, size_class_exp: u8) -> Option<u16> {
        // Get size class's stack metadata
        let fs_idx = usize::from(size_class_exp);
        
        let size = self.free_segments_sizes[fs_idx];
        let head_ptr = &mut self.free_segments_head_idxs[fs_idx];
        let data = self.free_segments[fs_idx];
        
        // Check not empty
        if size == 0 {
            return None;
        }

        // Pop from data
        let item = data[*head_ptr];
        *head_ptr = (*head_ptr + 1) % (FREE_STACK_MAX - 1);
        
        Some(item)
    }

    /// Updates a size class's free_segments stack based on the contents of a minipage's free_segments bitmap.
    /// If at least one free segment was found returns Some. The returned value is not pushed onto the stack (If only one value is found the stack could still be empty).
    /// Returns None if there were no free segments on the MiniPage.
    unsafe fn free_segments_update(&mut self, minipage: *mut MiniPageHeader) -> Option<u16> {
        let size_class = SizeClass::new((*minipage).size_class_exp);

        let mut search_byte_i = 0;
        let mut first_free_found: Option<u16> = None;

        for search_bit_i in 0..size_class.segments_max_num() {
            // Check if the bit corresponding to segment search_bit_i is marked as free
            let within_byte_bit_i = search_bit_i % 8;

            let search_byte = (*minipage).free_segments[search_byte_i];
            let search_mask = 1 << within_byte_bit_i;

            let bit_free_status = (search_byte & search_mask) >> within_byte_bit_i;
            if bit_free_status == 1 {
                // If first thing found, record to return
                if first_free_found == None {
                    first_free_found = Some(search_bit_i);
                } else {
                    // Not first found, push onto size class's stack
                    // Also ensure stack not full so we don't keep unnecessarily searching
                    if self.free_segments_push((*minipage).size_class_exp, search_bit_i) == None {
                        return first_free_found; // Exit
                    }
                }
            }
            
            // Check if last bit of the search byte, and need to retrieve next search byte from MiniPage's bitmap to look at in the next iteration
            if within_byte_bit_i == 7 {
                search_byte_i += 1;
            }
        }

        first_free_found
    }

    /// Setup a new MiniPageHead. Updates the next_minipage_addr, the free_lists head, and free_list_currents for the size class. Always adds the new MiniPageHead to the head of free_lists.
    /// Returns Option with the created MiniPage header if there was free space in the heap.
    /// Returns None if there is no space in the heap. This is fatal.
    unsafe fn add_minipage(&mut self, size_class_exp: u8) -> Option<*mut MiniPageHeader> {
        let size_class = SizeClass::new(size_class_exp);
        
        // Check there is room on the heap
        let max_allowed_addr = (*self.heap.get()).base_ptr().offset(isize::from(MAX_HOST_PAGES_ISIZE * heap::PAGE_BYTES_ISIZE));
        if self.next_minipage_addr >= max_allowed_addr {
            // Out of space on the host heap
            return None;
        }

        // Determine what the next node will be
        let mut next: Option<*mut MiniPageHeader> = None;
        if self.free_lists[size_class.exp_usize()] != null_mut() {
            next = Some(self.free_lists[size_class.exp_usize()]);
        }
          
        // Create new node
        let node_ptr = self.next_minipage_addr as *mut MiniPageHeader;
        (*node_ptr).next = next;
        (*node_ptr).size_class_exp = size_class_exp;
        (*node_ptr).free_segments = [1; 256]; // All 1 = all unallocated

        // Set size class's free list head to new node
        self.free_lists[size_class.exp_usize()] = node_ptr;

        // Set the size class's most recent node to new node
        self.free_list_currents[size_class.exp_usize()] = node_ptr;

        // Increment the next MiniPageHeader address
        self.next_minipage_addr = self.next_minipage_addr.offset(isize::from(MINI_PAGE_TOTAL_BYTES));

        Some(node_ptr)
    }

    /// Allocate memory.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let base_ptr = (*self.heap.get()).base_ptr();
        
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
            let delta_pages = MAX_HOST_PAGES - current_pages;

            assert!(delta_pages > 0, "Shouldn't be requesting to grow the memory by a negative number");
            
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

            // Save the first location we can put a MiniPage
            self.next_minipage_addr = (*self.heap.get()).base_ptr();

            self.did_init_heap = true;
        }

        // Determine size class of allocation
        let size_class = SizeClass::new_from_bytes(layout.size() as u16);

        // Check if not bigger than the largest MiniPage size class.
        // We don't do big alloc yet.
        if size_class.exp > MAX_SIZE_CLASS {
            return null_mut();
        }

        // Find the most recently used MiniPage header for this size class
        let mut node_ptr = self.free_list_currents[size_class.exp_usize()];

        // If no MiniPage for the size class was found
        if node_ptr == null_mut() {
            // This means we have to initialize the first MiniPage for this size class
            node_ptr = match self.add_minipage(size_class.exp) {
                Some(ptr) => ptr,
                None => {
                    // No space on host heap
                    null_mut()
                },
            };

            if node_ptr == null_mut() {
                // Host heap exausted
                return null_mut();
            }
        }

        assert!(node_ptr != null_mut(), "A MiniPageHeader should have been found at which to begin the search for a free segment to allocate");
        assert!(self.free_lists[size_class.exp_usize()] != null_mut(),"Since a MiniPageHeader to begin the search was found, the head of this size class's free free list should not be null");

        // Find the next free segment
        let next_free_segment_idx: Option<u16> = match self.free_segments_pop(size_class.exp) {
            Some(idx) => Some(idx), // The current MiniPage had free segments
            None => {
                // There are no free segments in the current MiniPage.
                
                // Search for a MiniPage of this size class which has free segments
                // Loop until search_head is a node with a free segment, then return.
                // Important to start the search at the head.
                let mut search_head: Option<*mut MiniPageHeader> = None;
                if self.free_lists[size_class.exp_usize()] != null_mut() {
                    search_head = Some(self.free_lists[size_class.exp_usize()]);
                }
                let mut found_idx: Option<u16> = None;
                while let Some(search_node) = search_head {
                    if let Some(idx) = self.free_segments_update(search_node) {
                        // Found at least one free segment, return for var
                        node_ptr = search_node;
                        found_idx = Some(idx);
                        break;
                    }

                    // Otherwise keep iterating
                    search_head = (*search_node).next;
                }

                match found_idx {
                    Some(idx) => Some(idx), // Found a MiniPage with free segments, one of which is idx
                    None => {
                        // If we get here then no MiniPages with free segments were found
                        // We need to setup a new MiniPage
                        let new_node = self.add_minipage(size_class.exp);
                        if let Some(new_node_ptr) = new_node {
                            // Record new_node as MiniPageHeader we are allocating from
                            node_ptr = new_node_ptr;

                            // Now add free segment indexes from brand new MiniPage onto size class's stack and return the first free segment from this MiniPage.
                            self.free_segments_update(new_node_ptr)
                        } else {
                            // Host heap exhausted
                            None
                        }
                    },
                }
            },
        };

        assert!(node_ptr == self.free_list_currents[size_class.exp_usize()], "MiniPageHeader free list header for this size class should also be the MiniPageHeader we are allocating from");
        assert!(node_ptr != null_mut(), "node_ptr should not be null");

        if let Some(free_segment_idx) = next_free_segment_idx {
            // Determine address we will allocate
            let page_addr = MemAddr::from_ptr(base_ptr, node_ptr as *mut u8);
            let page_meta = MiniPageMeta::from_addr(page_addr);
            let segment = page_meta.get_segment(size_class, usize::from(free_segment_idx));

            // Mark segment as not free
            segment.write_free_bitmap(base_ptr, false);

            // Return address
            return segment.as_addr().as_ptr(base_ptr);
        } else {
            // Failed to create a new MiniPageHeader, host heap full
            return null_mut();
        }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        let base_ptr = (*self.heap.get()).base_ptr();

        let addr = MemAddr::from_ptr(base_ptr, ptr);
        let page_meta = MiniPageMeta::from_addr(addr);

        // Read the size class
        let minipage_header_addr = MemAddr::new(page_meta.page_idx_u32() * MINI_PAGE_TOTAL_BYTES_U32);
        let minipage_header = minipage_header_addr.as_ptr(base_ptr) as *mut MiniPageHeader;

        let size_class = SizeClass::new((*minipage_header).size_class_exp);

        // Determine segment
        let segment = addr.get_segment(size_class);

        // Update segment bitmap
        segment.write_free_bitmap(base_ptr, true);
        // TODO: Push onto stack
    }
}

/// The custom global allocator. Wraps the AllocatorImpl
/// which performs all memory operations. See for more details.
pub struct AlligatorAlloc<H> where H: HostHeap {
    /// Data structure which keeps state of all memory
    /// wrapped inside an UnsafeCell for
    /// memory symantics.
    heap: UnsafeCell<AllocatorImpl<H>>,
}

/// WASM is single threaded right now so this should be okay.
unsafe impl<H> Sync for AlligatorAlloc<H> where H: HostHeap {}

impl AlligatorAlloc<HeapType> {
    pub const INIT: AlligatorAlloc<HeapType> = AlligatorAlloc{
        heap: UnsafeCell::new(AllocatorImpl::INIT),
    };
}

unsafe impl<H> GlobalAlloc for AlligatorAlloc<H> where H: HostHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        return (*self.heap.get()).alloc(layout);
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        (*self.heap.get()).dealloc(ptr, layout);
    }
}
