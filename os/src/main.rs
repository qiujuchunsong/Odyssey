// 移除标准库依赖
#![no_main]
#![no_std]

// 引入所用的宏
#[macro_use]
mod console;
mod config;
mod loader;
mod timer;
mod lang_items;
mod sbi;
mod sync;
pub mod task;
pub mod syscall;
pub mod trap;

// 引入global_asm! 宏
// 引入entry.asm 和 link_app.S 汇编文件
use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

// 地址获取函数
macro_rules! linker_symbol_address {
    ($symbol:path) => {
        ($symbol as *const ()).addr()
    };
}

#[unsafe(no_mangle)]
pub fn rust_main() -> !{
   
    unsafe extern "C" {
        safe fn stext(); // begin addr of text segment
        safe fn etext(); // end addr of text segment
        safe fn srodata(); // start addr of Read-Only data segment
        safe fn erodata(); // end addr of Read-Only data ssegment
        safe fn sdata(); // start addr of data segment
        safe fn edata(); // end addr of data segment
        safe fn sbss(); // start addr of BSS segment
        safe fn ebss(); // end addr of BSS segment
        safe fn boot_stack_lower_bound(); // stack lower bound
        safe fn boot_stack_top(); // stack top
    } 
    clear_bss();                    // 清除.bss段（上机后必需操作）
    trap::init();                   // 初始化trap
    loader::load_apps();            // 加载应用
    trap::enable_timer_interrupt(); // 使能时钟中断
    timer::set_next_trigger();      // 设置下次时钟中断
    task::run_first_task();         // 运行第一个任务
    panic!("Unreachable in rust_main!")
}

fn clear_bss() {
    unsafe extern "C" {
        safe fn sbss();
        safe fn ebss();
    }
    // 等价写法：用 for 循环逐字节清零（语义与下方 for_each 完全一致）
    // for a in linker_symbol_address!(sbss)..linker_symbol_address!(ebss) {
    //     unsafe { (a as *mut u8).write_volatile(0) }
    // }
    (linker_symbol_address!(sbss)..linker_symbol_address!(ebss)).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    })
}