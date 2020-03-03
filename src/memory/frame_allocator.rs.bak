use crate::consts::MAX_PHYSICAL_PAGES;

pub struct SegmentTreeAllocator {
    nodes: [u8; MAX_PHYSICAL_PAGES << 1],
    leaf_begin: usize,
    usable_num: usize,
    usable_offset: usize
}

impl SegmentTreeAllocator {
    fn child_l(idx: usize) -> usize { (idx << 1) + 1 }
    fn child_r(idx: usize) -> usize { (idx << 1) + 2 }
    fn parent(idx: usize) -> usize { if idx == 0 { 0 } else { ((idx - 1) >> 1) } }
    fn update_parents(&mut self, idx: usize) {
        let mut t = idx;
        while t > 0 {
            let p = SegmentTreeAllocator::parent(t);
            self.nodes[p] = self.nodes[SegmentTreeAllocator::child_l(p)] & 
                self.nodes[SegmentTreeAllocator::child_r(p)];
            t = p;
        }
    }
    // Initialize usable physical pages [l, r)
    pub fn init(&mut self, l: usize, r: usize) {
        assert!(r > l);
        self.usable_offset = l;
        self.usable_num = r - l;
        self.leaf_begin = 1;
        while self.leaf_begin < self.usable_num {
            self.leaf_begin = self.leaf_begin << 1;
        }
        self.leaf_begin -= 1;
        for i in (0..((self.leaf_begin << 1) + 1)) { self.nodes[i] = 1; }
        for i in (0..(self.usable_num)) { self.nodes[self.leaf_begin + i] = 0; }
        for i in (0..self.leaf_begin).rev() { 
            self.nodes[i] = self.nodes[SegmentTreeAllocator::child_l(i)] & 
                self.nodes[SegmentTreeAllocator::child_r(i)]; 
        }
    }
    // allocate a physical page from the left most unused page
    pub fn alloc(&mut self) -> usize {
        // assume we will not run out of memory
        if self.nodes[0] == 1 {
            panic!("Memory: Phisical memory run out!");
        }
        let mut p = 1;
        while p < self.usable_num {
            p = if self.nodes[SegmentTreeAllocator::child_l(p)] == 0 { 
                SegmentTreeAllocator::child_l(p) 
            } else { 
                SegmentTreeAllocator::child_r(p) 
            };
        }
        let result = p - self.leaf_begin + self.usable_offset;
        self.nodes[p] = 1;
        self.update_parents(p);
        result
    }
    // deallocate physical page
    pub fn dealloc(&mut self, idx: usize) {
        let mut p = idx - self.usable_offset + self.leaf_begin;
        assert!(self.nodes[p] == 1);
        self.nodes[p] = 0;
        self.update_parents(p);
    }
}

use spin::Mutex;

pub static SEGMENT_TREE_ALLOCATOR: Mutex<SegmentTreeAllocator> 
    = Mutex::new(SegmentTreeAllocator {
        nodes: [0; MAX_PHYSICAL_PAGES << 1],
        leaf_begin: 0,
        usable_num: 0,
        usable_offset: 0
    });

