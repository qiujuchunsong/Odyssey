//! 内核堆分配器（给内核自己用 `Vec` / `Box` / `BTreeMap` 等）
//!
//! 与 frame_allocator 的分工（两者最容易混）：
//!   - 本文件：把 .bss 里的一块静态数组(HEAP_SPACE)交给伙伴系统，
//!     供【内核代码自身】动态分配 —— 底层就是内核映像的一部分
//!   - frame_allocator：管理 [ekernel, MEMORY_END) 的【物理页】，
//!     供页表、用户程序使用 —— 底层是机器的物理内存
//!
//! 为什么必须有一个全局分配器：
//!   `alloc` crate（提供 Vec/Box/BTreeMap）要求整个程序恰好注册一个
//!   `#[global_allocator]`，否则链接时报 "no global memory allocator found"。

use buddy_system_allocator::LockedHeap;
use crate::config::KERNEL_HEAP_SIZE;
use core::ptr::addr_of_mut;

/// 全局堆分配器实例（伙伴系统）
///
/// 为什么能直接放进 static：`LockedHeap` 内部就是"自旋锁 + 空闲链表"，
/// 不依赖任何外部状态，因此可以 const 初始化（`empty()` 是 const fn）。
/// ⚠️ `empty()` 只是「占位」：此刻堆里【一字节都没有】，
///    必须等 `init_heap()` 把 HEAP_SPACE 交进去之后才能分配。
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// 内核堆的本体：一块 3MiB 的静态数组，直接占在 .bss 里。
/// （所以调大它会让内核映像变大；它也会被 clear_bss 清零）
///
/// 为什么是 `static mut`，后面还要用 `addr_of_mut!` 取地址：
///   - `static mut` 表达的是"这块内存会被可变访问"
///   - 但 edition 2024 禁止对 `static mut` 取引用（`static_mut_refs` 是硬错误：
///     `&static mut` 无法保证不被别名 → UB）
///   - 所以只能"取地址、不造引用"：`addr_of_mut!(HEAP_SPACE)`
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// 初始化堆：把 HEAP_SPACE 这段内存交给伙伴系统管理
///
/// ⚠️ 调用时机有硬约束：必须在【任何】用到堆的代码之前
///   （`Vec` / `Box`，以及带动态分配的 `lazy_static!`，例如 TASK_MANAGER）
///   否则会命中一个"空堆" → 触发 alloc_error_handler → panic。
///   所以 main.rs 的顺序是：clear_bss() → mm::init()(→ 这里) → 其他一切
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(core::ptr::addr_of_mut!(HEAP_SPACE) as usize, KERNEL_HEAP_SIZE);
    }
}


/// 堆自测：验证"分配出来的对象确实落在 .bss 的堆区间内"
///   - Box / Vec 的地址落在 [sbss, ebss) 之间 → 证明用的就是 HEAP_SPACE 那块内存
///   - Vec 连续 push 500 个元素 → 证明伙伴系统能正常扩展
#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    unsafe extern "C" {
        safe fn sbss();
        safe fn ebss();
    }
    let bss_range = sbss as usize..ebss as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}

/// 堆耗尽时的兜底（由 crate 根的 `#![feature(alloc_error_handler)]` 启用）
///
/// 什么时候触发：分配器返回 null —— 这里就是 HEAP_SPACE 用完了
/// 为什么直接 panic：内核没有"返回 ENOMEM 给用户"这一层，
/// 继续跑下去只会更糟 —— 当场暴露问题是唯一合理的选择
#[alloc_error_handler]
pub fn handle_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}