#![no_main]
#![no_std]
#![feature(start)]
#![feature(allocator_api)]
#![feature(strict_provenance)]
extern crate alloc;

use alloc::vec;
use core::panic::PanicInfo;
use core::ptr;
use windows_sys::Win32::System::Memory::{GetProcessHeap, HeapAlloc, HeapFree};

#[cfg(any(
    target_arch = "x86",
    target_arch = "arm",
    target_arch = "mips",
    target_arch = "mips32r6",
    target_arch = "powerpc",
    target_arch = "csky",
    target_arch = "powerpc64"
))]
const MIN_ALIGN: usize = 8;
#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "loongarch64",
    target_arch = "mips64",
    target_arch = "mips64r6",
    target_arch = "s390x",
    target_arch = "sparc64"
))]
const MIN_ALIGN: usize = 16;

unsafe impl alloc::alloc::GlobalAlloc for LocalAllocator {
    unsafe fn alloc(&self, layout: ::core::alloc::Layout) -> *mut u8 {
        unsafe { allocate(layout, true) }
    }

    unsafe fn alloc_zeroed(&self, layout: ::core::alloc::Layout) -> *mut u8 {
        unsafe { allocate(layout, true) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: ::core::alloc::Layout) {
        let block = {
            if layout.align() <= MIN_ALIGN {
                ptr
            } else {
                unsafe { ptr::read((ptr as *mut Header).sub(1)).0 }
            }
        };

        let heap = unsafe { GetProcessHeap() };
        unsafe { HeapFree(heap as _, 0, block as _) };
    }
}

#[inline]
fn init_or_get_process_heap() -> *mut u8 {
    unsafe { GetProcessHeap() as _ }
}

#[repr(C)]
struct Header(*mut u8);

#[inline]
unsafe fn allocate(layout: ::core::alloc::Layout, zeroed: bool) -> *mut u8 {
    let heap = init_or_get_process_heap();
    if heap.is_null() {
        return ptr::null_mut();
    }

    let flags: u32 = if zeroed { 0x00000008 } else { 0 };

    if layout.align() <= 8 {
        unsafe { HeapAlloc(heap as _, flags, layout.size()) as *mut u8 }
    } else {
        let total = layout.align() + layout.size();
        let ptr = unsafe { HeapAlloc(heap as _, flags, total) as *mut u8 };
        if ptr.is_null() {
            return ptr::null_mut();
        }
        let offset = layout.align() - (ptr.addr() & (layout.align() - 1));
        let aligned = unsafe { ptr.add(offset) };
        unsafe { ptr::write((aligned as *mut Header).sub(1), Header(ptr)) };
        aligned
    }
}
#[global_allocator]
static mut HEAPY: LocalAllocator = LocalAllocator {};

struct LocalAllocator {}

#[no_mangle]
#[start]
pub extern "C" fn main() -> u32 {
    let mut x = vec![1, 2, 3, 4, 5];

    x.push(6);
    x.push(7);
    x.push(9);

    assert_eq!(x.len(), 8);

    0
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

// See https://github.com/Trantect/win_driver_example/issues/4
#[no_mangle]
static _fltused: i32 = 0;
