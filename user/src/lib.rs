#![no_std]
#![feature(linkage)]    // 使用linkage,允许使用弱链接

#[macro_use]            // 引入宏,允许在当前模块中使用其他模块定义的宏
pub mod console;        // 引入console,用于处理用户态的输入输出
mod lang_items;         // 引入lang_items,用于处理Rust语言的特定项
mod syscall;            // 引入syscall,用于处理系统调用

// 该宏获取指定符号的地址，并将其转换为 usize 类型。它接受一个路径参数 $symbol，表示要获取地址的符号。宏内部将符号转换为指向空类型的指针 (*const ())，然后调用 addr() 方法获取其地址，并返回该地址作为 usize 类型。
macro_rules! linker_symbol_address {
    ($symbol:path) => {
        ($symbol as *const ()).addr()
    };
}

// main 使用 weak linkage: 每个 bin 提供强符号 main 覆盖这里
#[linkage = "weak"]
#[unsafe(no_mangle)]    // 禁止编译器对函数名进行修改,确保函数名在编译后保持不变
fn main() -> i32 {
    panic!("Cannot found main!");
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]     // 将_start函数置于.text.entry段，保证其位于内核的入口
extern "C" fn _start() -> ! {
    clear_bss();                            // 清理bss(启动必须操作)
    exit(main());                           // 调用每个bin里的main(),然后exit()
    panic!("Unreachable after sys_exit!");
}

fn clear_bss() {
    unsafe extern "C" {
        safe fn start_bss();
        safe fn end_bss();
    }
    (linker_symbol_address!(start_bss)..linker_symbol_address!(end_bss)).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}

use syscall::*;         // 使用syscall，该模块封装了sys_write()和sys_exit()

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}