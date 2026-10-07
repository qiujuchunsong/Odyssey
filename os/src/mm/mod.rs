//! 内存管理模块（mm）的门面（facade）：声明子模块 + 决定"对外暴露什么"。
//!
//! 为什么需要这层门面：下面 5 个 `mod xxx;` 全是【私有】的 ——
//! 外部无法通过 `crate::mm::page_table` 这类路径访问（文件结构属于内部实现），
//! 只能走这里精心列出的 re-export。所以本文件 = mm 对外的 API 清单。

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

// pub use 代表所引用的对象还会被外部调用者调用，如translated_byte_buffer会被syscall/fs.rs中的sys_write()调用
// use 代表调用对象仅在本层调用
pub use address::{PhysAddr, VirtAddr, PhysPageNum, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{FrameTracker, frame_alloc};
pub use memory_set::remap_test;
pub use memory_set::{KERNEL_SPACE, MapPermission, MemorySet};
use page_table::{PTEFlags, PageTable};
pub use page_table::{PageTableEntry, translated_byte_buffer, translated_str, translated_refmut};

/// 内存管理初始化。三步顺序不可交换：
///   ① init_heap            —— 之后内核才能用 Vec/Box（PageTable 里的 Vec<FrameTracker> 依赖它）
///   ② init_frame_allocator —— 页表页 / 数据页的来源
///   ③ activate             —— 激活内核地址空间；此后内核就在自己的页表里运行了
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();

}