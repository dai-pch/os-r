use crate::sbi::set_timer;
use riscv::register::{
    time,
    sie
};

// interrupt numbers of the timer
pub static mut TICKS: usize = 0;

// intervals of timer
// typically it's 1% of cpu frequency
static TIMEBASE: u64 = 100000;

pub fn init() {
    unsafe {
        TICKS = 0;
        sie::set_stimer(); // enable STIE
    }
    clock_set_next_event();
    println!("Timer: Init done.")
}

pub fn clock_set_next_event() {
    set_timer(get_cycle() + TIMEBASE);
}

fn get_cycle() -> u64 {
    time::read() as u64
}

