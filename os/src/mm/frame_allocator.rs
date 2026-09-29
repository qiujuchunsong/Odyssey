//! 帧分配器

use super::{PhysAddr, PhysPageNum};
use crate::config::MEMORY_END;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

/// 结构体FrameTracker用来管理物理页帧
/// RAII(Resource Acquisition Is Initialization)思想体现
/// 将实际的资源（一片物理页帧）与对象（FrameTracker）的生命周期绑定。资源在对象构造时获取，在对象析构时释放。
/// 使资源的管理更安全更高效
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    // 接管一页并清零（真正的分配发生在 alloc() / frame_alloc()）
    pub fn new(ppn: PhysPageNum) -> Self {
        let bytes_array = ppn.get_bytes_array();
        // 循环写0来清零该页
        // 必须清零，两类理由：
        //   页表项：残留的bit会被当做有效页表项进而翻译出乱七八糟的地址
        //   数据页：残留的上一个使用者的数据，清零放置数据泄露
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

/// 自定义Debug,调试时一眼分辨是谁的错误
impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

/// 使用Drop 做到析构对象时释放资源
impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

/// 帧分配器的三个特性：
/// 新建 -> new()
/// 分配 -> alloc() -> Option<> 可能能分到也可能分不到
/// 回收 -> dealloc()
/// 抽象成trait与:"使用者"解耦
trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// "栈式"分配器：实现极简
///   current..end  = 从未分配过的页号区间
///   recycled      = 被归还页号的【栈】(LIFO)：优先复用最近释放的页
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else {
            if self.current == self.end {
                None
            } else {
                self.current += 1;
                Some((self.current - 1).into()) 
            }
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // 不变量守卫（fail fast）：只允许归还"确实分配出去且尚未归还"的页。
        // 违反的两类 bug 都很隐蔽，与其静默放过、不如当场崩：
        //   ppn >= current        → 释放了从未分配的页（比如把别的地址当页号）
        //   recycled 里已存在      → 重复释放同一页（会导致两个所有者指向同一页）
        if ppn >= self.current || self.recycled
            .iter()
            .find(|&v| {*v == ppn})
            .is_some() {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        
        self.recycled.push(ppn);
    }
}


impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}

/// 目前的分配器实现。写成类型别名是为了"换实现只改这一行"（如换成 bitmap / buddy）
type FrameAllocatorImpl = StackFrameAllocator;
// 全局唯一实例：用 UPSafeCell 而不是 static mut ——
// 单核下用"运行期借用检查"（RefCell）来保证独占访问，避免 static mut 的 UB 风险
lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> = unsafe {
        UPSafeCell::new(FrameAllocatorImpl::new())
    };
}

/// 初始化可用区间 = [内核结束(ekernel 向上取整), MEMORY_END 向下取整)
/// 为什么从 ekernel 开始：ekernel 之前是内核自身（.text/.rodata/.data/.bss，含 3MB 堆），
/// 分配器绝不能把它们当空闲页发出去，否则内核会"把自己分配掉"
pub fn init_frame_allocator() {
    unsafe extern "C" {
        safe fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

/// 分配一页： 返回None 表示物理内存耗尽
/// 这里不做 unwrap —— 把"耗尽了怎么办"的决定权留给调用方
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(|ppn| FrameTracker::new(ppn))
}

fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR
        .exclusive_access()
        .dealloc(ppn);
}


#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}