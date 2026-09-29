OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x10000;

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    /* ★ 每段之间必须页对齐（ch4 新增要求，不能省）：
     *   内核的 MemorySet::from_elf() 是【按页】建映射的：
     *     段范围 = [vaddr.floor(), (vaddr+memsz).ceil())  → 会向外扩到整页
     *   如果两个段挤在同一页里（例如 .text 结束于 0x111d8、.rodata 从 0x111d8 开始），
     *   那么两个段的"页范围"都会包含 VPN 0x11 → 第二次 map 时触发内核断言
     *   `vpn ... is mapped before mapping`。
     *   所以：每个段都从新的一页开始，段之间永不共享页。 */
    . = ALIGN(4K);
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    .bss : {
        start_bss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        end_bss = .;
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}