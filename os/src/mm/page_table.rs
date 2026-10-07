//! 页表：把「虚拟页号 → 物理页号」这层翻译的【唯一入口】
//!
//! 本文件只干三件事：
//!   1. `PageTableEntry`：一个 8 字节的 PTE（低 8 位是标志位 | [53:10] 是 PPN）
//!   2. `PageTable`：一个 3 级页表的【所有者和查询器】，Sv39 三层，每层 512 项
//!   3. `translated_byte_buffer`：把不可信的用户指针翻译成内核能读写的切片
//! 别的模块（memory_set）不直接碰 PTE 的位运算，一律通过 `map` / `unmap` / `translate`。

// 兄弟模块通过super::调用
use super::{FrameTracker, PhysPageNum, StepByOne, PhysAddr, VirtAddr, VirtPageNum, frame_alloc};
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;
// 引入bitflags，用于创建页表项标志位
use bitflags::*;

bitflags! {
    /// 页表项的 8 个标志位（PTE的低8位；高位是PPN）
    /// 
    /// 必须由软件设置的四位：
    ///   V 有效 · R 可读 · W 可写 · X 可执行
    /// 剩下三位软件不主动设置，但【必须定义】
    pub struct PTEFlags: u8 {
        const V = 1 << 0; // Valid： 为1时表示该页表项合法 
        const R = 1 << 1; // Read 可读
        const W = 1 << 2; // Write 可写
        const X = 1 << 3; // eXecute 可执行
        const U = 1 << 4; // User：U态可访问  
        const G = 1 << 5; // Global： 切 satp 时 TLB 不失效
        const A = 1 << 6; // Accessed： 被访问过
        const D = 1 << 7; // Dirty： 被修改过
    }
}

#[derive(Copy, Clone)]
#[repr(C)] // 禁止编译器重排字段/改变布局 —— PTE 是硬件按裸 8 字节直接读的，布局必须稳定
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    /// 新建页表项（低10位留给flags，所以 << 10）
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// 清空页表项，防止数据泄露或污染内存
    pub fn empty() -> Self {
        PageTableEntry {
            bits: 0,
        }
    }
    /// SV39分页模式下：[53:10]为物理页号故右移动10位
    /// (1usize << 44) - 1 得到0x0FFF_FFFF_FFFF 与页表项右移10位得到的结果按位与就是物理地址
    /// .into()转换位物理页号
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        /// unwrap()的前提： 8个标志位全部定义齐全
        /// 否则返回None → panic
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// 判断对应标志位是否为1
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R != PTEFlags::empty())
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W != PTEFlags::empty())
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X != PTEFlags::empty())
    }
}

/// 页表结构体
/// FrameTracker体现了RAII思想，将资源与对象的生命周期绑定在一起
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

impl PageTable {
    /// 创建页表
    /// .unwrap()表示可能创建成功（有足够内存）也可能创建失败
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// 从 satp（即 `token()` 的返回值）反推出根页表，构造一个【临时视图】
    ///
    /// 用途：内核手里只有"用户地址空间的 token"（如 `current_user_token()`），
    /// 却需要按【用户页表】去翻译用户传来的指针 —— 这时就造一个 PageTable 来查表。
    /// 调用方：translated_byte_buffer（→ sys_write 翻译用户 buf）。
    ///
    /// `satp & ((1<<44)-1)`：satp 的布局是 MODE[63:60] | ASID[59:44] | PPN[43:0]，
    /// 只有低 44 位是根页表页号（MODE=8 表示 Sv39），所以用掩码丢掉高位。
    ///
    /// ⚠️ `frames: Vec::new()` 是故意留空的 —— 这是它与 `new()` 的本质区别：
    ///     new()        → 【拥有】页表页（frames 里装着 FrameTracker，drop 时归还）
    ///     from_token() → 只是"借来看"（不拥有任何页，drop 时绝不会拆掉别人的页表）
    ///   同一个 PageTable 类型，两种所有权语义，区别只在于 frames 是否为空。
    ///   代价：这个视图的有效性依赖调用方保证"那个地址空间还活着"。
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// 查找页表项_中间级断了则新建中间级的页表项，返回一个Option表示可能找得到也可能找不到
    /// 
    /// 用途：供map()使用
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();               // 将虚拟页号转化为三级页表的三个下标，供下文遍历
        let mut ppn = self.root_ppn;            // 游标：当前正在遍历的那一级页表，从根开始
        let mut result: Option<&mut PageTableEntry> = None; // 可能有结果也可能没结果
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {                 // i == 2 （第三级 = 叶子），查找成功
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {            // 中间级无效（V == 0）分配并继续查询
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V); // 中间级PTE只能带V，若R/W/X有任意一位=1则被视为叶子（若在第 1/2 级就是"大页"）→当场翻译，不再往下走
                self.frames.push(frame);    // 必须push，有drop逐一归还。否则出现静默内存泄漏（不报错，只是内存慢慢少）
            }
            ppn = pte.ppn();    // 向下一级
        }
        result
    }
    /// 查找页表项_中间级断了则返回None，返回一个Option表示可能找得到也可能找不到
    /// 
    /// 用途：供unmap()使用
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {    // 中间级无效（V == 0），查找失败返回None
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// 建立映射：让 `vpn` 指向物理页 `ppn`，权限为 `flags`
    ///
    /// 不变量（assert 守着）：该 vpn 必须【尚未映射】。
    /// 若静默覆盖，原物理页就失去最后一个引用 → 泄漏；报错能让 bug 早暴露。
    /// `flags | PTEFlags::V`：有效位由本函数统一负责，调用方不必操心。
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();   // 首次映射 ⇒ 顺路把中间级页表页建出来
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    /// 解除映射：把该 vpn 的页表项清成无效
    ///
    /// 不变量：该 vpn 必须【已映射】（否则说明上层算错了页号）。
    /// ⚠️ 只管"断开映射"，【不归还物理页】—— 页的回收靠持有 FrameTracker 的一方 drop。
    #[allow(unused)] // 调用者（MapArea::unmap / shrink_to）在 ch4 里都还走不到
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// 查询 vpn 的映射结果，返回 PTE 的【拷贝】
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            //println!("translate_va:va = {:?}", va);
            let aligned_pa: PhysAddr = pte.ppn().into();
            //println!("translate_va:pa_align = {:?}", aligned_pa);
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
    /// 生成该页表的 satp 值：写进 satp CSR，或存进 TrapContext.kernel_satp 供切页表用
    ///
    /// `8usize << 60`：satp 高 4 位是 MODE，8 = Sv39（0 = 关闭分页）；低 44 位是根页表页号。
    /// 与 `from_token()` 互为逆运算：一个拼、一个拆。
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// 按 `token` 所指的页表，把用户指针 `ptr..ptr+len` 翻译成内核可直接读写的一组切片
///
/// 为什么不直接解引用用户指针：用户页带 U 位，S 态（内核）访问会触发页错误；
/// 而且用户指针的合法性完全不可信 —— 必须走页表这层"翻译 + 校验"。
/// 为什么返回 Vec 而不是一个切片：一段用户数据可能【跨页且物理不连续】，
/// 每个物理页只能单独给出一段，调用方要自己遍历。
///
/// ⚠️ `&'static mut` 是"撒谎"出来的生命周期（物理内存的 'static）；
///   真正的保证是"调用期间该地址空间还在、且没有别人同时改它"，这两条由调用方负责。
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);   // 只借来查表，不拥有任何页（见 from_token）
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {                              // 一页一段地收集
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();              // 起始地址所在的那一页
        let ppn = page_table.translate(vpn).unwrap().ppn();  // unwrap：用户给了未映射的指针就会 panic
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();       // 本页的下一页首
        end_va = end_va.min(VirtAddr::from(end));    // 但不能越过 len 的范围
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);  // 正好取到页尾
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table.translate_va(VirtAddr::from(va)).unwrap().get_mut());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}

/// 把用户指针 `ptr`（类型 `*mut T`）翻译成内核可写的一块 `&mut T`
///
/// 谁在用：`sys_waitpid` 把子进程的 exit_code 写回用户态那个指针 ——
/// 用户指针不能直接解引用（用户页带 U 位，且内容完全不可信），必须走页表翻译。
/// 为什么返回 `&'static mut`：与 translated_byte_buffer 同样的"撒谎"（物理内存的 'static）；
/// 真正的保证是"调用期间该地址空间还在、且没有别人同时改它"，由调用方负责。
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let page_table = PageTable::from_token(token);   // 只借来查表，不拥有任何页（见 from_token）
    let va = ptr as usize;
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .get_mut()
}
