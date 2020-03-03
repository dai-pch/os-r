use riscv::register::{
    stvec,
    sscratch,
    sstatus,
    scause::{
        self,
        Trap,
        Exception,
        Interrupt
    }
};
use crate::context::TrapFrame;
use crate::timer::{
    TICKS,
    clock_set_next_event
};

global_asm!(include_str!("trap/trap.asm"));

pub fn init() {
    unsafe {
        extern "C" {
            fn __alltraps();
        }
        sscratch::write(0); // used to distinguish s-mode interrupt and u-mode interrupt.
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
        sstatus::set_sie();
    }
    println!("Interrupt: Init done.");
}

#[no_mangle]
pub fn rust_trap(tf: &mut TrapFrame) {
    let cause = tf.scause.cause();
    // println!("interrupt cause: {:?}", cause);
    match cause {
        Trap::Exception(Exception::Breakpoint) => breakpoint(&mut tf.sepc),    
        Trap::Interrupt(Interrupt::SupervisorTimer) => super_timer(),    
        _ => undefined_trap(tf)
    }
}

fn undefined_trap(tf: &mut TrapFrame) -> ! {
    let cause = tf.scause.cause();
    let epc = tf.sepc;
    println!("Unhandled trap {:?} @0x{:x}", cause, epc);
    panic!()
}

fn breakpoint(sepc: &mut usize) {
    println!("Breakpoint is activate @0x{:x}", sepc);
    *sepc += 2;
}

fn super_timer() {
    clock_set_next_event();
    unsafe {
        TICKS += 1;
        if ((TICKS + 1) % 100 == 0) {
            println!("100 ticks passed.")
        }
    }
}

