pub trait DynamicAllocator {
    fn alloc(&mut self, size: usize, align: usize) -> Option<usize>;
    fn dealloc(&mut self, addr: usize);
    fn grained(&self, minsz: usize) -> usize;
    fn compound_head(&mut self, addr: usize) -> usize;
}

pub fn next_pow_of_2(x: usize) -> usize {
    if x == 0 { return x }
    1 << (8 * (core::mem::size_of::<usize>()) - (x - 1).leading_zeros() as usize)
}

pub fn prev_pow_of_2(x: usize) -> usize {
    if x == 0 { return x }
    1 << (8 * (core::mem::size_of::<usize>()) - x.leading_zeros() as usize - 1)
}

