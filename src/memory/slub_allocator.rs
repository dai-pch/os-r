use core::mem::size_of;
use core::cmp::{min, max};
use crate::memory::allocator::{
    DynamicAllocator,
    prev_pow_of_2,
    next_pow_of_2
};

const slub_pool_sizes: [usize; 15] = 
    [8, 16, 24, 32, 48, 64, 92, 128, 192, 256, 384, 512, 768, 1024, 2048];
    //, 4096, 8*1024, 16*1024, 32*1024, 64*1024],

pub struct SlubAllocator<T: DynamicAllocator> {
    slub_pools: [Option<SlubPool<T>>; slub_pool_sizes.len()],
    back_allocator_p: *mut T,
}

impl<T: DynamicAllocator> SlubAllocator<T> {
    pub fn new(back_allocator_p: *mut T) -> Self {
        let mut slub_pools: [Option<SlubPool<T>>; slub_pool_sizes.len()] = Default::default();
        for i in (0..slub_pool_sizes.len()) {
            slub_pools[i] = Some(SlubPool::new(back_allocator_p, slub_pool_sizes[i]));
        }
        SlubAllocator {
            slub_pools,
            back_allocator_p
        }
    }
    pub const fn max_size() -> usize {
        slub_pool_sizes[slub_pool_sizes.len() - 1]
    }
    fn pool_id_from_size(&self, minsz: usize) -> usize {
        assert!(minsz <= Self::max_size());
        let mut id = 0;
        while slub_pool_sizes[id] < minsz { id += 1; }
        id
    }
    fn frame_from_addr(&mut self, addr: usize) -> *mut SlubFrame<T> {
        let head = unsafe { (*(self.back_allocator_p)).compound_head(addr) };
        head as *mut SlubFrame<T>
    }
    fn pool_from_frame(&mut self, frame: *mut SlubFrame<T>) -> *mut SlubPool<T> {
        unsafe { (*frame).pool_ptr }
    }
}

impl<T: DynamicAllocator> DynamicAllocator for SlubAllocator<T> {
    fn alloc(&mut self, size: usize, align: usize) -> Option<usize> {
        if size > Self::max_size() { 
            return unsafe { (*(self.back_allocator_p)).alloc(size, align) }; 
        }
        let pool_id = self.pool_id_from_size(size);
        let pool = self.slub_pools[pool_id].as_mut().unwrap();
        pool.alloc(align)
    }
    fn dealloc(&mut self, addr: usize) {
        let frame_p = self.frame_from_addr(addr);
        if frame_p as usize == addr {
            // if the frame head equals to addr, 
            // means this memory block is allocated by
            // back_allocator.
            unsafe { (*(self.back_allocator_p)).dealloc(addr) };
            return;
        }
        let pool_p = self.pool_from_frame(frame_p);
        unsafe {
            (*pool_p).dealloc(addr);
        }
    }
    fn grained(&self, minsz: usize) -> usize { 
        if minsz > Self::max_size() { 
            unsafe { (*(self.back_allocator_p)).grained(minsz) }
        } else {
            slub_pool_sizes[self.pool_id_from_size(minsz)]
        }
    }
    fn compound_head(&mut self, addr: usize) -> usize {
        let frame_p = self.frame_from_addr(addr);
        if frame_p as usize == addr {
            // if the frame head equals to addr, 
            // means this memory block is allocated by
            // back_allocator.
            return frame_p as usize;
        }
        let pool_p = self.pool_from_frame(frame_p);
        unsafe { (*pool_p).compound_head(frame_p as usize, addr) }
    }
}

struct SlubPool<T: DynamicAllocator> {
    grained: usize,
    frame_size: usize,
    real_blk_size: usize,
    blk_offset: usize,
    back_allocator_p: *mut T,
    current_frame: Option<*mut SlubFrame<T>>,
    full_frame_list: Option<*mut SlubFrame<T>>,
    partial_frame_list: Option<*mut SlubFrame<T>>
}

impl<T: DynamicAllocator> SlubPool<T> {
    pub fn new(back_allocator_p: *mut T, grained: usize) -> Self {
        let frame_size = unsafe { (*back_allocator_p).grained(grained*16) };
        let real_blk_size = max(grained, size_of::<SlubBlk<T>>());
        let blk_offset = Self::align(size_of::<SlubFrame<T>>(), real_blk_size);
        SlubPool {
            grained,
            frame_size,
            real_blk_size,
            blk_offset,
            back_allocator_p,
            current_frame: None,
            full_frame_list: None,
            partial_frame_list: None
        }
    }
    pub fn alloc(&mut self, align: usize) -> Option<usize> {
        let mut res: Option<usize> = None;
        if let Some(frame_p) = self.current_frame {
            unsafe {
                if !(*frame_p).is_full() {
                    res = unsafe { (*frame_p).alloc(align) };
                    return res;
                } else {
                    self.current_frame = None;
                    self.insert_to_full(frame_p);
                }
            }
        } else {
            if let Some(partial_first) = self.partial_frame_list {
                self.drop_from_partial(partial_first);
                self.current_frame = Some(partial_first);
            } else {
                let new_frame = self.alloc_frame();
                if let Some(new_frame_p) = new_frame {
                    self.current_frame = Some(new_frame_p);
                } else {
                    return None;
                }
            }
        }
        self.alloc(align)
    }
    pub fn dealloc(&mut self, addr: usize) {
        let frame_p = (addr & !(self.frame_size - 1)) as *mut SlubFrame<T>;
        unsafe {
            let is_full = (*frame_p).is_full();
            (*frame_p).dealloc(addr);
            if is_full {
                self.move_from_full_to_partial(frame_p);
                return;
            }
            if self.current_frame != Some(frame_p) && (*frame_p).is_empty() {
                self.drop_from_partial(frame_p);
                unsafe { (*(self.back_allocator_p)).dealloc(frame_p as usize) };
            }
        }
    }
    fn align(x: usize, align: usize) -> usize {
        let a = 1 << align.trailing_zeros();
        let mask = a - 1;
        (x + mask) & !mask
    }
    fn alloc_frame(&mut self) -> Option<*mut SlubFrame<T>> {
        let new_alloc = unsafe { (*(self.back_allocator_p)).alloc(self.frame_size, 1) };
        if let Some(new_frame_addr) = new_alloc {
            let new_frame_p = new_frame_addr as *mut SlubFrame<T>;
            unsafe {
                (*new_frame_p).init(
                    self as *mut SlubPool<T>, 
                    new_frame_addr, 
                    self.frame_size, 
                    self.real_blk_size, 
                    self.blk_offset
                );
            }
            Some(new_frame_p)
        } else { 
            None 
        }
    }
    fn insert_to_full(&mut self, frame_p: *mut SlubFrame<T>) {
        unsafe {
            if let Some(full_first_p) = self.full_frame_list {
                (*full_first_p).last_frame = Some(frame_p);
            }
            (*frame_p).next_frame = self.full_frame_list;
            (*frame_p).last_frame = None;
            self.full_frame_list = Some(frame_p);
        }
    }
    fn drop_from_partial(&mut self, frame_p: *mut SlubFrame<T>) {
        unsafe {
            if let Some(last_p) = (*frame_p).last_frame {
                (*last_p).next_frame = (*frame_p).next_frame;
            } else {
                self.partial_frame_list = (*frame_p).next_frame;
            }
            if let Some(next_p) = (*frame_p).next_frame {
                (*next_p).last_frame = (*frame_p).last_frame;
            }
            (*frame_p).next_frame = None;
            (*frame_p).last_frame = None;
        }
    }
    fn move_from_full_to_partial(&mut self, frame_p: *mut SlubFrame<T>) {
        unsafe {
            // drop from full
            if let Some(last_p) = (*frame_p).last_frame {
                (*last_p).next_frame = (*frame_p).next_frame;
            } else {
                self.full_frame_list = (*frame_p).next_frame;
            }
            if let Some(next_p) = (*frame_p).next_frame {
                (*next_p).last_frame = (*frame_p).last_frame;
            }
            // insert to partial
            if let Some(partial_first_p) = self.partial_frame_list {
                (*partial_first_p).last_frame = Some(frame_p);
            }
            (*frame_p).next_frame = self.partial_frame_list;
            (*frame_p).last_frame = None;
            self.full_frame_list = Some(frame_p);
        }
    }
    pub fn compound_head(&mut self, frame_start: usize, addr: usize) -> usize {
        (addr - frame_start - self.blk_offset) / self.real_blk_size * self.real_blk_size
    }
} 

struct SlubFrame<T: DynamicAllocator> {
    pub next_frame: Option<*mut SlubFrame<T>>,
    pub last_frame: Option<*mut SlubFrame<T>>,
    pub pool_ptr: *mut SlubPool<T>,
    free_blks: Option<*mut SlubBlk<T>>,
    in_use: usize
}

struct SlubBlk<T: DynamicAllocator> {
    pub next_blk: Option<*mut SlubBlk<T>>
}

impl<T: DynamicAllocator> SlubFrame<T> {
    pub fn init(&mut self, pool: *mut SlubPool<T>, start: usize, 
                frame_size: usize, real_blk_size: usize, offset: usize) {
        self.next_frame = None;
        self.last_frame = None;
        self.pool_ptr = pool;
        self.in_use = 0;
        let blk_start = start + offset;
        self.free_blks = Some(blk_start as *mut SlubBlk<T>);
        let mut p = unsafe { self.free_blks.unwrap() };
        let mut np = blk_start + real_blk_size;
        unsafe { 
            while np <= start + frame_size - real_blk_size {
                unsafe { (*p).next_blk = Some(np as *mut SlubBlk<T>); }
                p = unsafe { (*p).next_blk.unwrap() };
                np += real_blk_size;
            }
            // p = (*p).next_blk.unwrap();
            (*p).next_blk = None; 
        }
    }
    pub fn alloc(&mut self, align: usize) -> Option<usize> {
        assert!(!self.is_full());
        let mut blk_p = self.free_blks.unwrap();
        if blk_p as usize % align == 0 {
            self.free_blks = unsafe { (*blk_p).next_blk };
            self.in_use += 1;
            return Some(blk_p as usize);
        }
        let mut blk = unsafe { (*blk_p).next_blk };
        while let Some(blk_p) = blk {
            let next: Option<*mut SlubBlk<T>> = unsafe { (*blk_p).next_blk };
            if let Some(next_p) = next {
                if next_p as usize % align == 0 {
                    unsafe { (*blk_p).next_blk = (*next_p).next_blk };
                    self.in_use += 1;
                    return Some(next_p as usize);
                }
            }
            blk = next;
        }
        None
    }
    pub fn dealloc(&mut self, addr: usize) {
        let p = addr as *mut SlubBlk<T>;
        unsafe { (*p).next_blk = self.free_blks.take(); }
        self.free_blks = Some(p);
        self.in_use -= 1;
    }
    pub fn is_empty(&self) -> bool {
        self.in_use == 0
    }
    pub fn is_full(&self) -> bool {
        self.free_blks.is_none()
    }
}

