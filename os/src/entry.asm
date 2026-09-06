    .section .text.entry    # kernel entry code section
    .globl _start           # export _start as global symbol (program entry)
_start:
    la sp, boot_stack_top   # set stack pointer to top of boot stack
    call rust_main          # jump to Rust entry point

    .section .bss.stack     # boot stack lives in BSS (zero-initialized)
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16        # 16 pages (4KB each) reserved for the stack
    .globl boot_stack_top
boot_stack_top:
