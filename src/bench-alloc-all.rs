mod alloc;
use alloc::{AlligatorAlloc,MIN_SIZE_CLASS,MAX_SIZE_CLASS};
use alloc::heap::HeapType;
use core::alloc::Layout;
use std::alloc::GlobalAlloc;
use cfg_if::cfg_if;

/*
 * What follows is the benchmark program. Right now it
 * just tries to get any sort of memory allocation
 * to occur. Comment out annotation but keep variable if 
 * debugging ALLOC crashes further.
 */
// #[global_allocator]
static ALLOC: AlligatorAlloc<HeapType> = AlligatorAlloc::INIT;

/// Allocate then free a number of bytes for each size class which will require more than one MiniPage.
unsafe fn alloc_all() {
    for n in MIN_SIZE_CLASS..=MAX_SIZE_CLASS {
        let mut free_later: Vec<*mut u8> = vec!();

        let segments_per_page = 2048 / 2_u32.pow(u32::from(n));

        for i in 0..segments_per_page * 10 {
            // Create layout which requests the maximum number of bytes possible for this size class
            let layout = match Layout::from_size_align(2_usize.pow(u32::from(n)), 1) {
                Ok(l) => l,
                Err(e) => panic!("error making Layout: {}", e),
            };

            // Call allocate
            let ptr = ALLOC.alloc(layout);

            // Ensure the allocation succeeded
            cfg_if! {
                if #[cfg(feature = "metrics")] {
                    if ptr.is_null() {
                        eprintln!("alloc failure cause={:?}", ALLOC.alloc_failure_cause());
                    }
                }
            }
            assert!(!ptr.is_null(), "alloc() failed (returned null): size class={}, i={}", n, i);

            // For 1/6th of allocations don't free them immediately, free them later
            if i % 6 == 0 {
                free_later.push(ptr);
            } else {
                ALLOC.dealloc(ptr, layout);
            }
        }

        // Free the memory we intentionally left laying around.
        for ptr in free_later.iter() {
            let layout = match Layout::from_size_align(2_usize.pow(u32::from(n)), 1) {
                Ok(l) => l,
                Err(e) => panic!("error making Layout: {}", e),
            };
            
            ALLOC.dealloc(*ptr, layout);
        }
    }

    // Show statistics about run
    println!("fresh / reused stats: {:?}", ALLOC.fresh_reused_stats());
    
    cfg_if! {
        if #[cfg(feature = "metrics")] {

            println!("metrics: {:?}", ALLOC.metrics());

        }
    }
    
    println!("done");
}

/// Allocate and free a lot of times.
fn main() {
    for i in 0..1 {
        println!("Benchmark iteration {}", i);
        unsafe {
            alloc_all();
        }
    }
}
