/* 链接脚本：控制内核映像在内存中的布局
 *
 * ch4 相比 ch3 的改动只有 4 处（都在下面用 ★ 标出）：
 *   ① BASE_ADDRESS 用回内核基址 0x80200000（不是 user 侧的 0x10000）
 *   ② 把 .text.trampoline（跳板代码）单独成段并对齐到 4K —— 跳板必须独占一页
 *   ③ 新增 sbss_with_stack：.bss 中「包含内核启动栈」的那部分起点
 *   ④ ekernel（内核映像结束）ch3 已有 —— 页帧分配器从它开始分配物理页
 */
OUTPUT_ARCH(riscv)          /* 指定目标架构为 RISC-V */
ENTRY(_start)              /* 程序入口点符号，启动汇编代码从这里开始执行 */
BASE_ADDRESS = 0x80200000; /* ★① 内核镜像的基址：OpenSBI 把内核加载到这里 */

SECTIONS
{
    . = BASE_ADDRESS;    /* 当前地址指针设置为基址 */
    skernel = .;         /* 定义内核开始地址符号 skernel */

    stext = .;           /* 定义代码段开始地址符号 stext */
    .text : {            /* .text 段，存放可执行代码 */
        *(.text.entry)   /* 入口代码段，比如 startup 或 _start */

        /* ★② 跳板（trap.s 里的 __alltraps / __restore）必须独占一页：
         *   理由1：trap_return() 要用 `__restore - __alltraps + TRAMPOLINE`
         *          算出它在跳板页中的地址 —— 前提是两者【在同一页内且相对偏移固定】
         *   理由2：这一页要被【内核页表】和【每个应用的页表】都映射到同一个物理页，
         *          这样切换 satp 的瞬间取指才不会失败
         *   所以流程是：先对齐到 4K → 记下 strampoline → 放入跳板 → 再对齐到 4K，
         *   保证这段代码不与任何其他代码共用一页。 */
        . = ALIGN(4K);
        strampoline = .;
        *(.text.trampoline);
        . = ALIGN(4K);

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

    /* ★③ sbss_with_stack：.bss 里【含内核启动栈】的那部分起点
     *   为什么 bss 需要两个起点符号？
     *     - sbss_with_stack：memory_set::new_kernel() 用它把整个 .bss（含栈）
     *                        恒等映射成可读可写 —— 内核栈也得能读写
     *     - sbss           ：clear_bss() / heap_test() 用它（从「栈之后」开始清零，
     *                        避免把正在使用的启动栈清掉） */
    sbss_with_stack = .;
    .bss : {
        *(.bss.stack)    /* 内核启动栈（entry.asm 里的 boot_stack）放最前 */
        sbss = .;        /* bss 段开始地址符号 sbss（不含栈） */
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);       /* .bss 段结束后再次对齐 */
    ebss = .;            /* bss 段结束地址符号 ebss */
    ekernel = .;         /* ★④ 内核映像结束地址：页帧分配器从这里开始分配物理页 */

    /DISCARD/ : {        /* 不放入最终镜像的节 */
        *(.eh_frame)     /* 异常处理/栈展开信息，对内核通常不需要 */
        /* 注意：这里【故意不丢弃】.debug* —— 因为内核要用 GDB 调试（make debug），
         *       丢了 DWARF 信息就没法按源码断点了。（user 侧的 linker.ld 才丢它） */
    }
}