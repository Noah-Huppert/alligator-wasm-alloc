mod alloc;

/*
 * What follows is the benchmark program. Right now it
 * just tries to get any sort of memory allocation
 * to occur.
 */
#[global_allocator]
static ALLOC: alloc::AlligatorAlloc = alloc::AlligatorAlloc::INIT;

fn main() {
    for i in 0..100000 {
        greet(&format!("Alligator wasmtime, i={}", i));
    }
}

fn greet(name: &str) {
    println!("hello {}", name);
}
