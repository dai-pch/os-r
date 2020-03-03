extern crate spin;

use core::ops::Deref;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use crate::memory::allocator::DynamicAllocator;
use crate::memory::buddy_allocator::BuddyAllocator;

pub struct MutexedAllocator<T: DynamicAllocator>(Mutex<T>);

impl<T: DynamicAllocator> MutexedAllocator<T> {
    pub const fn new(c: T) -> Self {
        MutexedAllocator(Mutex::new(c)) 
    }
}

impl<T: DynamicAllocator> Deref for MutexedAllocator<T> {
    type Target = Mutex<T>;

    fn deref(&self) -> &Mutex<T> {
        &self.0
    }
}

unsafe impl<T: DynamicAllocator> GlobalAlloc for MutexedAllocator<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let res = self.lock().alloc(layout.size(), layout.align()).unwrap();
        res as *mut u8
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock().dealloc(ptr as usize);
    }
}

pub type MutexedBuddyAllocator<'a> = MutexedAllocator<BuddyAllocator<'a>>;

