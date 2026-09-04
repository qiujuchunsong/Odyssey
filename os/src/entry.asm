    .section .text.entry    // 定义代码段(.section .text.entry),用于存放程序入口代码
    .globl _start           // 声明全局符号_start,作为程序入口点
_start:
    la sp, boot_stack_top   // load address: 将boot_stack_top的地址加载到栈指针寄存器sp中,设置栈顶
    call rust_main          // 调用rust_main函数,开始执行Rust代码

    .section .bss.stack     // 定义未初始化数据段(.section .bss.stack),用于存放栈空间
    .globl boot_stack_lower_bound   // 声明全局符号boot_stack_lower_bound,作为栈的下界
boot_stack_lower_bound:
    .space 4096 * 16        // 分配16个页面(每页4KB)的空间,用于栈
    .globl boot_stack_top   // 声明全局符号boot_stack_top,作为栈的上界
boot_stack_top: