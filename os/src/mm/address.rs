//! 4个newtype + 地址<->页号 + VPNRange迭代


use super::PageTableEntry;                      // 兄弟模块统一从super::取
use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS}; 
use core::fmt::{self, Debug, Formatter};        // self→为了写 fmt::Result；Debug/Formatter 供下面 4 个手写 impl Debug 用

const PA_WIDTH_SV39: usize = 56;                // Sv39 物理地址宽 56 位 = 44位的物理页号 + 12位的页内偏移
const VA_WIDTH_SV39: usize = 39;                // RISC-V架构中虚拟地址仅有39位
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;   // 44位页号
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;   // 27位 = 3级 * 9

/// 物理地址
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

/// 虚拟地址
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

/// 物理页号
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

/// 虚拟页号
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

/// 重构Debug，使调试页表时一眼分辨这是哪种地址
impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}
impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VPN:{:#x}", self.0))
    }
}
impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PA:{:#x}", self.0))
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PPN:{:#x}", self.0))
    }
}

// ========== usize → newtype：打包（按位宽截断） ==========
// 四种类型的有效位数不同（物理地址 56 / 物理页号 44 / 虚拟地址 39 / 虚拟页号 27），
// 从 usize 构造时按各自位宽截断并丢弃高位。
impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        Self(v & ((1 << PA_WIDTH_SV39) - 1))
    }
}
impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self {
        Self(v & ((1 << PPN_WIDTH_SV39) - 1))
    }
}
impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {
        Self(v & ((1 << VA_WIDTH_SV39) - 1))
    }
}
impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {
        Self(v & ((1 << VPN_WIDTH_SV39) - 1))
    }
}

// ========== newtype → usize：解包 ==========
// 页号与物理地址没有"规范形式"的概念，解包即原样返回内部值。
// 例外是 VirtAddr —— 它必须做符号扩展，见下。
impl From<PhysAddr> for usize {
    fn from(v: PhysAddr) -> Self {
        v.0
    }
}
impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self {
        v.0
    }
}
// Sv39 要求 39 位虚拟地址的高 25 位必须等于第 38 位：
//   第 38 位 = 0（低半区/用户）→ 高位补 0
//   第 38 位 = 1（高半区/内核）→ 高位补 1
impl From<VirtAddr> for usize {
    fn from(v: VirtAddr) -> Self {
        if v.0 >= (1 << (VA_WIDTH_SV39 - 1)) {
            v.0 | (!((1 << VA_WIDTH_SV39) - 1)) // 补1
        } else {
            v.0
        }
    }
}
impl From<VirtPageNum> for usize {
    fn from(v: VirtPageNum) -> Self {
        v.0
    }
}

impl VirtAddr {
    /// 向下取整到"本地址所在的页"
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }
    /// 向上取整到"覆盖本地址所需的下一个页边界"
    /// 与 floor 配对，把 [start, end) 的【地址区间】变成
    /// [floor(start), ceil(end)) 的【页区间】—— 首尾未对齐的页也要整页映射
    pub fn ceil(&self) -> VirtPageNum {
        if self.0 == 0 {
            VirtPageNum(0)
        } else {
            VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
        }
    }
    /// 页内偏移（低12位）
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// 是否页对齐
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
} 
// 地址 -> 页号：丢掉页偏移，故使用断言要求页对齐 
impl From<VirtAddr> for VirtPageNum {
    fn from(v: VirtAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
// 页号 → 地址：得到"这一页的首地址"（× 4096），无损
impl From<VirtPageNum> for VirtAddr {
    fn from(v: VirtPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
} 

// 计算物理地址页偏移
impl PhysAddr {
    pub fn page_offset(&self) -> usize { self.0 & (PAGE_SIZE - 1) }
}
// 地址 -> 页号：丢掉页偏移，故使用断言要求页对齐 
impl From<PhysAddr> for PhysPageNum {
    fn from(v: PhysAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
// 页号 → 地址：得到"这一页的首地址"（× 4096），无损
impl From<PhysPageNum> for PhysAddr {
    fn from(v: PhysPageNum) -> Self { Self(v.0 << PAGE_SIZE_BITS) }
}

impl PhysAddr {
    // 向上取整
    pub fn floor(&self) -> PhysPageNum { PhysPageNum(self.0 / PAGE_SIZE) }
    // 向下取整
    pub fn ceil(&self) -> PhysPageNum { PhysPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE) }
    /// 把一个【带页内偏移的物理地址】（translate_va 算出来的那种）当作 T 访问。
    ///
    /// 与下面 `PhysPageNum::get_mut` 的分工：那个只会从"页首"开始，
    /// 这个能带偏移 —— 因为 translated_str / translated_refmut 要翻译的是
    /// 任意用户指针（比如 exit_code 指针），它不一定落在页首。
    /// 隐含前提与 get_bytes_array 那段完全相同：恒等映射 + 该物理页没人回收 +
    /// size_of::<T>() 不越界，三条都由调用者负责。
    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
}

/// 把 27 位虚拟页号拆成三级页表的下标：`[根表行号, 二级表行号, 叶子表行号]`
///
/// Sv39 三级页表每级 512 项（一页 4096B ÷ 8B/PTE = 512），行号需要 9 位，
/// 而 27 位 VPN 恰好 = 3 × 9，所以天然分三段（按位从高到低）：
///     VPN[2] → 根表，VPN[1] → 二级表，VPN[0] → 叶子表
///
/// 循环从【最低位】开始取（`& 511` 永远取最低 9 位），
/// 所以先取到的那段填到最大下标 idx[2]，最终 idx[0] 就是最高 9 位（根表行号）。
/// 例：VPN 0x11 → [0, 0, 17]；VPN 0x7FFFF（TRAMPOLINE）→ [1, 511, 511]
///
/// 调用方：PageTable::find_pte / find_pte_create（逐级向下查页表）
impl VirtPageNum {
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 511;
            vpn >>= 9;
        }
        idx
    }
}

// ==========把"物理页号"变成"可直接访问的内存"==========
// 共同点：先用 From 把 PPN 转成物理地址（页首），再用 from_raw_parts_mut 造引用。
//
// 为什么返回 &'static mut:
//      这段内存是物理内存，整个程序运行期间都有效
//      使用'static而不用裸指针 *mut u8 是为了减少unsafe的使用，让上层代码干净
// 【隐含前提 —— 违反任何一条都是 UB 或 PageFault】
//   1. 恒等映射：KERNEL_SPACE 把 [ekernel, MEMORY_END) 映射成了 VA = PA，
//      所以物理地址可以直接当指针解引用（去掉这段映射，这里立刻 PageFault）
//   2. 所有权：这个 PPN 确实由某个 FrameTracker 持有，没被回收/复用
//   3. 粒度：get_pte_array 按"512 项"用、get_bytes_array 按"4096 字节"用
//      （一页 4096B ÷ 8B/PTE = 512，这两个数与 PAGE_SIZE 必须一致）
//   4. 别名：'static mut 等于跳过借用检查 —— 连续调用会得到多个指向同一页的
//      &mut，安全性由调用者负责（别让两个引用同时活着并写同一页）
// 谁在用：get_pte_array → find_pte；get_bytes_array → copy_data/FrameTracker::new；
//        get_mut → trap_cx_ppn.get_mut()（放 TrapContext）
// into():
//      .into()是编写impl From<PhysPageNum> for PhysAddr白送的
//      将物理页号转换为物理地址
// core::slice::from_raw_parts_mut():
//      把一个(裸指针，元素个数)变成:"可变切片"引用
impl PhysPageNum {
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) } // 一张页有512个页表项
    }
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) } // 4096 = PAGE_SIZE：这一页的全部字节
    }
    /// 把这一页的**起始处**当作一个 T 访问：T 由调用者决定
    /// 不检查 size_of::<T>() ≤ 4096，由调用者保证。
    /// 这里 as_mut().unwrap() 不会失败 —— 0 号物理页从不被分配（分配器从 ekernel 开始）
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
}

pub trait StepByOne {
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone)]
/// a simple range structure for type T
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}
impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
}
impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}
/// iterator for the simple range structure
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}
impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}
impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}

/// a simple range structure for virtual page number
pub type VPNRange = SimpleRange<VirtPageNum>;
