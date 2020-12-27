use std::alloc::{System, GlobalAlloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

struct LoggingAllocator;


unsafe impl GlobalAlloc for LoggingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        print!("Allocating {:?} bytes", layout.size());
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static A: LoggingAllocator = LoggingAllocator;