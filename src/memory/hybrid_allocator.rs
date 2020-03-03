use core::marker::Send;
use crate::memory::allocator::DynamicAllocator;
use crate::memory::buddy_allocator::BuddyAllocator;
use crate::memory::slub_allocator::SlubAllocator;

pub struct HybridAllocator<'a> {
    back: BuddyAllocator<'a>,
    front: Option<SlubAllocator<BuddyAllocator<'a>>>
}

unsafe impl<'a> Send for HybridAllocator<'a> {}

impl<'a> HybridAllocator<'a> {
    pub const fn new() -> Self {
        let back = BuddyAllocator::<'a>::new();
        HybridAllocator {
            back,
            front: None
        }
    }
    pub fn init(&mut self, start: usize, size: usize) {
        self.back.init(start, size);
        self.front = Some(SlubAllocator::<BuddyAllocator<'a>>::new(&mut self.back as *mut BuddyAllocator<'a>));
    }
}

impl<'a> DynamicAllocator for HybridAllocator<'a> {
    fn alloc(&mut self, size: usize, align: usize) -> Option<usize> {
        if let Some(ref mut f) = self.front {
            f.alloc(size, align)
        } else {
            self.back.alloc(size, align)
        }
    }
    fn dealloc(&mut self, addr: usize) {
        if let Some(ref mut f) = self.front {
            f.dealloc(addr);
        } else {
            self.back.dealloc(addr);
        }
    }
    fn grained(&self, minsz: usize) -> usize {
        if let Some(ref f) = self.front {
            f.grained(minsz)
        } else {
            self.back.grained(minsz)
        }
    }
    fn compound_head(&mut self, addr: usize) -> usize {
        if let Some(ref mut f) = self.front {
            f.compound_head(addr)
        } else {
            self.back.compound_head(addr)
        }
    }
}

