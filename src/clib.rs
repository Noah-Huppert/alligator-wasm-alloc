mod alloc;

use core::alloc::Layout;
use std::alloc::GlobalAlloc;
use libc::size_t;
use std::ffi::c_void;

static ALLOC: alloc::AlligatorAlloc = alloc::AlligatorAlloc::INIT;

#[no_mangle]
pub unsafe extern "C" fn alligator_alloc(size: size_t) -> *mut c_void {
    let layout = match Layout::from_size_align(size, 1) {
        Ok(l) => l,
        Err(e) => panic!("error making Layout for alloc({}): {}", size, e),
    };
    ALLOC.alloc(layout) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn alligator_realloc(ptr: *mut c_void, new_size: size_t) -> *mut c_void {
    let layout = match Layout::from_size_align(0, 1) {
        Ok(l) => l,
        Err(e) => panic!("error making Layout for realloc({}, {}): {}", ptr as u32, new_size, e),
    };
    ALLOC.realloc(ptr as *mut u8, layout, new_size) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn alligator_dealloc(ptr: *mut c_void) {
    let layout = match Layout::from_size_align(0, 1) {
        Ok(l) => l,
        Err(e) => panic!("error making Layout for alloc({}): {}", ptr as u32, e),
    };
    ALLOC.dealloc(ptr as *mut u8, layout)
}

