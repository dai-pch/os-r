use riscv::register::{
    sstatus::Sstatus,
    scause::Scause
};

#[repr(C)]
pub struct TrapFrame {
    pub x: [usize; 32], // General Registers
    pub sstatus: Sstatus, // Supervisor Status Register
    pub sepc: usize, // Supervisor Exception Program Counter
    pub stval: usize, // Supervisor Trap Value
    pub scause: Scause, // Scause register
}

