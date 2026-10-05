#![no_std]
#![feature(linkage)]    // 使用linkage,允许使用弱链接
#![feature(alloc_error_handler)]

#[macro_use]            // 引入宏,允许在当前模块中使用其他模块定义的宏
pub mod console;        // 引入console,用于处理用户态的输入输出
mod lang_items;         // 引入lang_items,用于处理Rust语言的特定项
mod syscall;            // 引入syscall,用于处理系统调用

use buddy_system_allocator::LockedHeap;
use core::ptr::addr_of_mut;
use syscall::*;

const USER_HEAP_SIZE: usize = 16384;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

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
    // 【为什么必须在这里初始化堆】main() 的调用链里只要出现 String/Vec/Box，
    // 就会走 #[global_allocator] (= HEAP) 向它要内存；而 HEAP 声明时是
    // LockedHeap::empty() —— 一个 size=0 的空堆，此时任何分配都会
    // panic("memory allocation of N bytes failed")。
    // 所以顺序是死的: clear_bss() → 堆就绪 → main()。
    // 【为什么用 addr_of_mut! 而不是 &mut HEAP_SPACE】
    //   1) edition 2024 里 static_mut_refs 是 deny-by-default，取 &mut 直接编译不过；
    //   2) 这里只需要一个整数地址，不该凭空造出一个 &mut 引用(那是对别名规则的额外承诺)。
    // init() 之后 HEAP_SPACE 这一整块 static 数组就交给 buddy 分配器管理了。
    unsafe {
        HEAP.lock()
            .init(addr_of_mut!(HEAP_SPACE) as usize, USER_HEAP_SIZE);
    }
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

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

pub fn yield_() -> isize {
     sys_yield() 
}

pub fn get_time() -> isize {
    sys_get_time()
}

pub fn getpid() -> isize {
    sys_getpid()
}

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec(path: &str) -> isize {
    sys_exec(path)
}

pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {
                yield_();
           }
            exit_pid => return exit_pid,
        }
    }
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => {
                yield_();
           }
            exit_pid => return exit_pid,
        }
    }
}
