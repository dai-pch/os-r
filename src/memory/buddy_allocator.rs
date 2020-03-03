// use core::Option;
use core::cmp::max;
use core::slice;
use crate::memory::allocator::{
    DynamicAllocator,
    prev_pow_of_2,
    next_pow_of_2
};

pub const LOG_BUDDY_ALLOCATOR_GRANULARITY: usize = 12;
pub const BUDDY_ALLOCATOR_GRANULARITY: usize = (1 << LOG_BUDDY_ALLOCATOR_GRANULARITY);

pub struct BuddyAllocator<'a> {
    nodes: Option<&'a mut [u8]>,
    leaf_num: usize,
    addr_high: usize,
    rounded_size: usize
}

impl<'a> BuddyAllocator<'a> {
    pub const fn new() -> Self {
        BuddyAllocator {
            nodes: None,
            leaf_num: 0,
            addr_high: 0,
            rounded_size: 0
        }
    }

    pub fn init(&mut self, start: usize, size: usize) {
        self.rounded_size = next_pow_of_2(size) << 1;
        let mask = self.rounded_size - 1;
        self.addr_high = start & (!mask);
        self.leaf_num = self.rounded_size >> LOG_BUDDY_ALLOCATOR_GRANULARITY;
        if (size <= max(BUDDY_ALLOCATOR_GRANULARITY, self.leaf_num << 1) + BUDDY_ALLOCATOR_GRANULARITY) {
            panic!("The space managered by BuddyAllocator is too small.");
        }
        let nodes_p = start as *mut u8;
        let nodes: &'a mut [u8] = unsafe{ slice::from_raw_parts_mut(nodes_p, self.leaf_num << 1) };
        for i in (1..(self.leaf_num << 1)) { nodes[i] = 0; }
        for i in (0..self.leaf_num) { 
            let addr = self.addr_high | (i << LOG_BUDDY_ALLOCATOR_GRANULARITY);
            nodes[self.leaf_num + i] = 
                if addr <= (start + (self.leaf_num << 1)) || 
                    addr > (start + size - BUDDY_ALLOCATOR_GRANULARITY) 
                { 0 } else { 1 };
        }
        for i in (1..self.leaf_num).rev() { 
            let cl_blk_log = nodes[self.child_l(i)];
            let cr_blk_log = nodes[self.child_r(i)];
            if cl_blk_log == self.level(i) && cr_blk_log == self.level(i) {
                nodes[i] = self.level(i) + 1
            } else {
                nodes[i] = max(cl_blk_log, cr_blk_log);
            }
        }
        // println!("    nodes: {:?}", nodes);
        self.nodes = Some(nodes);
    }

    pub fn dealloc(&mut self, addr: usize) {
        if addr & (BUDDY_ALLOCATOR_GRANULARITY - 1) != 0 {
            panic!("Invalid addr to dealloc.");
        }
        let id = self.allocated_node_id(addr);
        // dealloc
        let nodes: &mut [u8] = self.nodes.take().unwrap();
        {
            let addr = self.node_addr(id);
            let size = self.node_total_blk(id) << LOG_BUDDY_ALLOCATOR_GRANULARITY;
            //println!("Buddy allocator: dealloc memory at 0x{:x} with size 0x{:x}.", addr, size);
        }
        nodes[id] = 1 + self.level(id);
        self.nodes = Some(nodes);
        self.update(id);
    }

    pub fn grained(&self, minsz: usize) -> usize {
        next_pow_of_2(minsz)
    }

    pub fn compound_head(&mut self, addr: usize) -> usize {
        let id = self.allocated_node_id(addr);
        self.node_addr(id)
    }

    fn find_alloc(&mut self, blk_n: usize, align: usize, id: usize) -> Option<usize> {
        let nodes = self.nodes.take().unwrap();
        // println!("    in find_alloc with id: {}, blk_n: {}, node_value: {}", id, blk_n, nodes[id]);
        if nodes[id] == 0 || (1 << (nodes[id] - 1)) < blk_n {
            self.nodes = Some(nodes);
            return None;
        }
        self.nodes = Some(nodes);
        if id >= self.leaf_num && self.node_addr(id) & (next_pow_of_2(align) - 1) == 0 {
            return Some(id);
        }
        let res_r = self.find_alloc(blk_n, align, self.child_r(id));
        if let Some(res) = res_r {
            return Some(res);
        }
        let res_l = self.find_alloc(blk_n, align, self.child_l(id));
        if let Some(res) = res_l {
            return Some(res);
        }
        let nodes = self.nodes.take().unwrap();
        if nodes[id] == self.level(id) + 1 && 
            self.node_addr(id) & (next_pow_of_2(align) - 1) == 0 {
            self.nodes = Some(nodes);
            return Some(id);
        }
        self.nodes = Some(nodes);
        None
    }

    fn do_alloc(&mut self, id: usize) {
        let nodes = self.nodes.take().unwrap();
        nodes[id] = 0;
        self.nodes = Some(nodes);
        self.update(id);
    }
    
    fn update(&mut self, idx: usize) {
        let nodes = self.nodes.take().unwrap();
        let mut t = idx;
        while t > 0 {
            let p = self.parent(t);
            let cl_blk_log = nodes[self.child_l(p)];
            let cr_blk_log = nodes[self.child_r(p)];
            if cl_blk_log == cr_blk_log && cl_blk_log == self.level(p) {
                nodes[p] = self.level(p) + 1;
            } else {
                nodes[p] = max(cl_blk_log, cr_blk_log);
            }
            t = p;
        }
        self.nodes = Some(nodes);
    }
    fn child_l(&self, id: usize) -> usize { id << 1 }
    fn child_r(&self, id: usize) -> usize { id << 1 | 1 }
    fn parent(&self, id: usize) -> usize { id >> 1 }
    fn level(&self, id: usize) -> u8 { 
        (id.leading_zeros() - self.leaf_num.leading_zeros()) as u8
    }
    fn node_addr(&self, id: usize) -> usize {
        let mut l_leaf_id = id;
        while l_leaf_id < self.leaf_num { l_leaf_id = self.child_l(l_leaf_id); }
        let offset = (l_leaf_id - self.leaf_num) << LOG_BUDDY_ALLOCATOR_GRANULARITY;
        self.addr_high | offset
    }
    fn node_total_blk(&self, id: usize) -> usize {
        1 << (self.level(id) + 1)
    }
    fn allocated_node_id(&mut self, addr: usize) -> usize {
        let nodes: &mut [u8] = self.nodes.take().unwrap();
        let offset_mask = self.rounded_size - 1;
        let offset = addr & offset_mask;
        let mut id = self.leaf_num + (offset >> LOG_BUDDY_ALLOCATOR_GRANULARITY);
        while nodes[id] != 0 { id = self.parent(id); }
        self.nodes = Some(nodes);
        id
    }
}

impl<'a> DynamicAllocator for BuddyAllocator<'a> {
    /*fn alloc(&mut self, size: usize, align: usize) -> Option<usize> {
        self.alloc(size, align)
    }*/
    fn alloc(&mut self, size: usize, align: usize) -> Option<usize> {
        let find_id = self.find_alloc(
            (size + BUDDY_ALLOCATOR_GRANULARITY - 1) >> LOG_BUDDY_ALLOCATOR_GRANULARITY,
            align,
            1
        );
        match find_id {
            Some(id) => { 
                self.do_alloc(id); 
                let addr = self.node_addr(id);
                {
                    //println!("Buddy allocator: alloc memory at 0x{:x} with size 0x{:x}.", 
                    //         addr, self.node_total_blk(id) << LOG_BUDDY_ALLOCATOR_GRANULARITY);
                }
                Some(addr) 
            }
            None => None
        }
    }


    fn dealloc(&mut self, addr: usize) {
        if addr & (BUDDY_ALLOCATOR_GRANULARITY - 1) != 0 {
            panic!("Invalid addr to dealloc.");
        }
        let id = self.allocated_node_id(addr);
        // dealloc
        let nodes: &mut [u8] = self.nodes.take().unwrap();
        {
            let addr = self.node_addr(id);
            let size = self.node_total_blk(id) << LOG_BUDDY_ALLOCATOR_GRANULARITY;
            //println!("Buddy allocator: dealloc memory at 0x{:x} with size 0x{:x}.", addr, size);
        }
        nodes[id] = 1 + self.level(id);
        self.nodes = Some(nodes);
        self.update(id);
    }

    fn grained(&self, minsz: usize) -> usize {
        next_pow_of_2(minsz)
    }

    fn compound_head(&mut self, addr: usize) -> usize {
        let id = self.allocated_node_id(addr);
        self.node_addr(id)
    }
}

impl<'a> Default for BuddyAllocator<'a> {
    fn default() -> Self { Self::new() }
}

