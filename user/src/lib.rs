#![no_std]
#![feature(linkage)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

macro_rules! linker_symbol_address {
    ($symbol:path) => {
        ($symbol as *const ()).addr()
    };
}

// main 使用 weak linkage: 每个 bin 提供强符号 main 覆盖这里
#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("Cannot found main!");
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
extern "C" fn _start() -> ! {
    clear_bss();
    exit(main());
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

use syscall::*;

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}