/* 链接脚本：控制内核映像在内存中的布局 */
OUTPUT_ARCH(riscv)          /* 指定目标架构为 RISC-V */
ENTRY(_start)              /* 程序入口点符号，启动汇编代码从这里开始执行 */
BASE_ADDRESS = 0x80200000; /* 内核镜像的基址，内核从这个地址开始加载 */

SECTIONS
{
    . = BASE_ADDRESS;    /* 当前地址指针设置为基址 */
    skernel = .;         /* 定义内核开始地址符号 skernel */

    stext = .;           /* 定义代码段开始地址符号 stext */
    .text : {            /* .text 段，存放可执行代码 */
        *(.text.entry)   /* 入口代码段，比如 startup 或 _start */
        *(.text .text.*) /* 其他所有 .text 节中的代码 */
    }

    . = ALIGN(4K);       /* 将下一个段对齐到 4KB 边界 */
    etext = .;           /* 代码段结束地址符号 etext */
    srodata = .;         /* 只读数据段开始地址符号 srodata */
    .rodata : {          /* .rodata 段，存放只读数据常量 */
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);       /* 对齐到下一页，方便内存管理 */
    erodata = .;         /* 只读数据段结束地址符号 erodata */
    sdata = .;           /* 已初始化数据段开始地址符号 sdata */
    .data : {            /* .data 段，存放已初始化的全局/静态变量 */
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);       /* 再次对齐，保证 .bss 段页对齐 */
    edata = .;           /* 已初始化数据段结束地址符号 edata */
    .bss : {
        *(.bss.stack)
        sbss = .;        /* bss 段开始地址符号 sbss */
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);       /* .bss 段结束后再次对齐 */
    ebss = .;            /* bss 段结束地址符号 ebss */
    ekernel = .;         /* 内核镜像结束地址符号 ekernel */

    /DISCARD/ : {        /* 不放入最终镜像的节 */
        *(.eh_frame)     /* 异常处理/栈展开信息，对内核通常不需要 */
    }
}
