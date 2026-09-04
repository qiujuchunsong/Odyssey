#![no_main]
#![no_std]
mod lang_items;
mod console;
mod sbi;

use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));

macro_rules! linker_symbol_address {
    ($symbol:path) => {
        ($symbol as *const ()).addr()
    };
}

#[unsafe(no_mangle)]
pub fn rust_main() -> !{
    clear_bss();
    println!("Hello World!");
    panic!("Shutdown Machine!");
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