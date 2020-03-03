mod allocator;
mod frame_allocator;
mod mutexed_allocator;
mod buddy_allocator;
mod slub_allocator;
mod hybrid_allocator;

use frame_allocator::SEGMENT_TREE_ALLOCATOR as FRAME_ALLOCATOR;
use riscv::addr::{
    VirtAddr,
    PhysAddr,
    Page,
    Frame
};

pub fn init(l: usize, r: usize) {
    FRAME_ALLOCATOR.lock().init(l, r);
    println!("Memory: Setup done.");
}

pub fn alloc_frame() -> Option<Frame> {
    Some(Frame::of_ppn(FRAME_ALLOCATOR.lock().alloc()))
}

pub fn dealloc_frame(f: Frame) {
    FRAME_ALLOCATOR.lock().dealloc(f.number());
}

use mutexed_allocator::MutexedAllocator;
use buddy_allocator::BuddyAllocator;
use hybrid_allocator::HybridAllocator;
use crate::consts::KERNEL_HEAP_SIZE;

#[global_allocator]
static KERNEL_DYNAMIC_ALLOCATOR: MutexedAllocator<HybridAllocator> = 
    MutexedAllocator::new(HybridAllocator::new());
    // MutexedAllocator::new(BuddyAllocator::new());

#[alloc_error_handler]
fn alloc_error_handler(_: core::alloc::Layout) -> ! {
    panic!("Dynamic allocation failed!");
}

pub fn init_heap() {
    static mut HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];
    println!("Initialize heap at 0x{:x} with size 0x{:x}.", 
             unsafe { &HEAP as *const _ as usize }, KERNEL_HEAP_SIZE);
    unsafe {
        KERNEL_DYNAMIC_ALLOCATOR
            .lock()
            .init(HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
    println!("Memory: Initializing heap done.")
}

