.equ XLENB, 8 # Byte numbers per register

# Load value at sp + 8 * a2 to register a1
.macro LOAD a1, a2
    ld \a1, \a2 * XLENB(sp)
.endm

# Store value of register a1 at sp+8*a2
.macro STORE a1, a2
    sd \a1, \a2 * XLENB(sp)
.endm

# Save all registers into stack
.macro SAVE_ALL
    # swap sp and sscratch atomatically
    csrrw sp, sscratch, sp

    # the interrupt is caused in s-mode
    # if sp == 0, that is sscratch == 0 before swap
    # thus we need to read kernel stack from sscratch back to sp 
    bnez sp, trap_from_user
trap_from_kernel:
    csrr sp, sscratch
trap_from_user:
    # allocate space from stack to save registers
    addi sp, sp, -36 * XLENB
    # x0 is no need to save, x2 is sp, which will processed later
    STORE x1, 1
    STORE x3, 3
    STORE x4, 4
    STORE x5, 5
    STORE x6, 6
    STORE x7, 7
    STORE x8, 8
    STORE x9, 9
    STORE x10, 10
    STORE x11, 11
    STORE x12, 12
    STORE x13, 13
    STORE x14, 14
    STORE x15, 15
    STORE x16, 16
    STORE x17, 17
    STORE x18, 18
    STORE x19, 19
    STORE x20, 20
    STORE x21, 21
    STORE x22, 22
    STORE x23, 23
    STORE x24, 24
    STORE x25, 25
    STORE x26, 26
    STORE x27, 27
    STORE x28, 28
    STORE x29, 29
    STORE x30, 30
    STORE x31, 31
    
    # save some csr
    csrrw s0, sscratch, x0
    STORE s0, 2
    csrr s1, sstatus
    STORE s1, 32
    csrr s2, sepc
    STORE s2, 33
    csrr s3, stval
    STORE s3, 34
    csrr s4, scause
    STORE s4, 35
.endm 

# RESTORE all registers from stack
.macro RESTORE_ALL
    LOAD s1, 32 # sstatus
    LOAD s2, 33 # sepc
# use SPP flag in sstatus to find out u-mode interrupt and s-mode interrupt
    andi s0, s1, 1 << 8
    bnez s0, _to_kernel
_to_user:
    addi s0, sp, 36 * XLENB
    csrw sscratch, s0
_to_kernel:
    csrw sstatus, s1
    csrw sepc, s2

    LOAD x1, 1
    LOAD x3, 3
    LOAD x4, 4
    LOAD x5, 5
    LOAD x6, 6
    LOAD x7, 7
    LOAD x8, 8
    LOAD x9, 9
    LOAD x10, 10
    LOAD x11, 11
    LOAD x12, 12
    LOAD x13, 13
    LOAD x14, 14
    LOAD x15, 15
    LOAD x16, 16
    LOAD x17, 17
    LOAD x18, 18
    LOAD x19, 19
    LOAD x20, 20
    LOAD x21, 21
    LOAD x22, 22
    LOAD x23, 23
    LOAD x24, 24
    LOAD x25, 25
    LOAD x26, 26
    LOAD x27, 27
    LOAD x28, 28
    LOAD x29, 29
    LOAD x30, 30
    LOAD x31, 31
    
    LOAD x2, 2
.endm 

    .section .text
    .global __alltraps
__alltraps:
    SAVE_ALL
    mv a0, sp
    jal rust_trap

    .global __trapret
__trapret:
    RESTORE_ALL
    sret

