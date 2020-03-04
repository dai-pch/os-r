    .section .text.entry
    .global _start
_start:
    # set page table item for kernel
    lui     t0, %hi(boot_page_table_sv39)
    li      t1, 0xffffffffc0000000 - 0x80000000
    sub     t0, t0, t1
    srli    t0, t0, 12

    li      t1, 8 << 60  # Sv39 flag of RISC-V
    or      t0, t0, t1
    csrw    satp, t0
    sfence.vma
    # set kernel stack
    # la      sp, bootstacktop
    lui     sp, %hi(bootstacktop)
    # call rust_main
    lui     t0, %hi(rust_main)
    addi    t0, t0, %lo(rust_main)
    jr      t0

    .section .bss.stack
    .align 12
    .global bootstack
bootstack:
    .space 4096 * 4
    .global bootstacktop
bootstacktop:

    .section .data
    .align 12
boot_page_table_sv39:
    # map 0xffffffffc0000000 to 0x80000000 (1GB)
    .zero 8 * 511
    # VRWXAD
    # .quad (0x80000 << 10) | 0xcf 
    .quad 0x200000cf  

