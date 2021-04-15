mod alloc;
use alloc::AlligatorAlloc;
use alloc::heap::HeapType;
// use core::alloc::Layout;
// use std::alloc::GlobalAlloc;

/*
 * What follows is the benchmark program. Right now it
 * just tries to get any sort of memory allocation
 * to occur. Comment out annotation but keep variable if 
 * debugging ALLOC crashes further.
 */
#[global_allocator]
static ALLOC: AlligatorAlloc<HeapType> = AlligatorAlloc::INIT;

fn main() {
    // Uncomment to debug ALLOC crashes further
    // unsafe {
    //     let layout = match Layout::from_size_align(4, 1) {
    //         Ok(l) => l,
    //         Err(e) => panic!("error making Layout: {}", e),
    //     };
        
    //     println!("alloc(): {:?}", ALLOC.alloc(layout));
    // }


    // Comment to debug ALLOC crashes further
    for i in 0..10000000 {
        greet(&format!("Alligator wasmtime, i={}", i));
    }
}

fn greet(name: &str) {
    println!("hello {}", name);
}
