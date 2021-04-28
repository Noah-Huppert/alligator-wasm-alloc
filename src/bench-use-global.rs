mod alloc;
use alloc::AlligatorAlloc;
use alloc::heap::HeapType;

/*
 * What follows is the benchmark program. Right now it
 * just tries to get any sort of memory allocation
 * to occur.
 */
#[global_allocator]
static ALLOC: AlligatorAlloc<HeapType> = AlligatorAlloc::INIT;

/// Allocates some of a particular size class using Alligator to manage the runtime's heap.
fn main() {
    let mut v: [Vec<bool>; 10] = [vec!(), vec!(), vec!(), vec!(), vec!(), vec!(), vec!(), vec!(), vec!(), vec!(), ];
    for x in 0..10 {
        for i in 0..1024 {
            v[x].push(true);
            unsafe {
                greet(&format!("Alligator wasmtime, x={}, i={}, fresh / reused stats={:?}", x, i, ALLOC.fresh_reused_stats()));
            }
        }
    }
}

fn greet(name: &str) {
    println!("hello {}", name);
}
