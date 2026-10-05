# Odyssey · Rust 生词本

> 用途：把"跟着 rCore 敲代码时遇到、但没系统学过的 Rust 写法"记下来，按主题归并。
> 记录原则：只记「名字 + 在哪出现 + 为什么需要它 + 坑」。不抄文档。
> 覆盖范围：ch1 ~ ch4（2026-08-31 ~ 2026-09-29）。之后遇到新生词继续往下加。

## 0. 怎么用这本生词本

- **按"我卡在哪"查**：不知道 `into()` 干什么 → 第 1 节；报 `traits need to be in scope` → 第 3 节；`static_mut_refs` 报错 → 第 6 节。
- 第 7 节是「**我以为我知道**」的坑合集 —— 都是这四章里真踩过的，比前面更值得看。
- 第 8 节是 ch5 会用到的新词，提前扫一眼混个脸熟即可。

---

## 1. 类型转换：`From` / `Into`（最重要的一节）

| 名字 | 出现位置 | 一句话 |
|---|---|---|
| `From` / `Into` / `.into()` / `T::from(v)` | `mm/address.rs` 全套 newtype | 类型间**显式**转换的 trait |
| `impl From<A> for B` | `address.rs` 的 8 个 impl | 写了它，就**免费**得到 `Into<B> for A` |

### 三条必须记住的

1. **`Into` 是 `From` 的免费反向**：`impl From<usize> for VirtAddr` 之后，`a.into()` 和 `VirtAddr::from(a)` 完全等价。
   所以**惯例是只写 `From`**，别手写 `Into`。
2. **本项目是一个 2×4 的转换矩阵**：`usize → {PhysAddr, PhysPageNum, VirtAddr, VirtPageNum}` 与反向，共 8 个。
3. ★ **两个方向的语义不对称**（ch4 隐蔽坑）：
   - `usize → newtype` = **按位宽截断**（56/44/39/27 位），静默丢高位、不检查范围 → 保证"类型内部永远满足自己的不变量"
   - `newtype → usize` = 3 个是恒等（直接取 `.0`），**只有 `VirtAddr → usize` 要做符号扩展**（canonicalization：高 25 位补成与第 38 位相同）。
     少了它，`TRAMPOLINE` 会被截成 `0x7FFF_F000`，写进 `stvec`/`satp` 就是**非规范地址** → 取指异常。
4. `into()` 能写出来、编译能推荐 `From::from` 的场景，说明**实现者已经把转换的语义定好了**；看到 `.into()` 不确定时，去 `impl From` 里看它是"截断"还是"符号扩展"。

---

## 2. 所有权、生命周期、RAII

| 名字 | 出现位置 | 一句话 |
|---|---|---|
| `Drop`（trait）+ RAII | `mm/frame_allocator.rs` 的 `FrameTracker` | 对象销毁时自动归还资源 |
| `#[derive(Copy, Clone, ...)]` | 4 个 newtype、`TaskStatus` | `Copy` = 按位拷贝（栈上 memcpy）；`Clone` = 显式 `.clone()` |
| `Arc<T>` | `KERNEL_SPACE`、ch4 `TaskControlBlock` | 共享所有权的引用计数指针（no_std 下来自 `alloc`） |
| `Weak<T>` | ch5 的父进程指针（待学） | 弱引用，**不增加计数** → 打破 `Arc` 循环 |
| `&'static mut [u8]` | `translated_byte_buffer` 的返回值 | 生命周期是"撒谎"出来的（物理内存的 `'static`），真保证由调用方负责 |
| `'_` | `Formatter<'_>`、`RefMut<'_, T>` | 生命周期占位，让编译器自己填 |

### 关键分水岭

- ★ **有堆资源的类型不能 `Copy`**：`FrameTracker` 拥有一个物理页（drop 时要归还），所以**只能 `Clone`**（或者干脆不实现）。这就是"`Vec<FrameTracker>` 是所有权清单"的由来。
- ★ **`PageTable` 的两种所有权语义**：`new()` 的 `frames` 里装着 `FrameTracker`（**拥有**页表页，drop 时归还）；`from_token()` 的 `frames: Vec::new()`（**只是借用**，drop 时绝不拆别人的页表）。同一个类型，区别只在 `frames` 是否为空。
- **`exclusive_access()` 返回的是"借用守卫"`RefMut<'_, T>`**，不是裸 `&mut`；它的**存活期到所在语句末尾** —— 所以链式写法里闭包还在"守卫仍持有"期间执行，闭包内再碰同一个 `UPSafeCell` 就会运行期 panic。

---

## 3. trait 是"能力"

**一句话心法：trait 不是"继承"，是"这个类型会做什么"。**

| 名字 | 出现位置 | 一句话 |
|---|---|---|
| `Debug` / `{:?}` | 到处 | 能被 `println!("{:?}", x)` 打印 |
| `Ord` / `PartialOrd` | `VirtAddr` 的 `.min()` | 能比较大小 → `a.min(b)` 可用 |
| `Iterator` / `IntoIterator` / `type Item` | `address.rs` 的 `SimpleRange`/`VPNRange` | 能 `for x in collection` |
| `FnOnce` / 闭包 / 函数项 | `Option::map(FrameTracker::new)` | 函数名本身也是值 |
| `unsafe impl Sync` | `sync/up.rs` 的 `UPSafeCell` | 手写"我保证它跨线程安全"（单核内核里是一种承诺） |

### ★ 最容易忘的一条：trait 方法调用需要 trait 在作用域内

`vpn.step()` 其实是 `StepByOne::step(&mut vpn)` 的糖。没 `use StepByOne` 就报：

```
E0599: no method named `step` found ... help: traits need to be in scope
```

**这就是"为什么有些 `use` 看起来没用却被引入"的答案。** 同类情形还有：
- 运算符重载（`use core::ops::Add` 后 `a + b` 才能用）
- 宏（`use alloc::vec;` 才让 `vec!` 可用）
- `bitflags::*`（带来 `from_bits`/`empty`/`bits`）

**判断 import 是否真无用 → 看 `unused_imports` 警告**（VS Code 里 rust-analyzer 会把无用 import 变灰），不要靠肉眼；类型藏在泛型参数里（如 `Vec<FrameTracker>`）也容易被漏看。

**反面教材（同一个机制）**：`use core::borrow::BorrowMut;` 让 `x.borrow_mut()` 被 blanket impl `BorrowMut<Self> for Self` 劫走 → 报 `no field ... on &mut UPSafeCell<...>`。

### ★ 迭代器适配器（惰性求值）

| 名字 | 出现位置 | 一句话 |
|---|---|---|
| `.map()` | `Option::map`、`Iterator::map` | "有值就变换"，不引入控制流 |
| `.rev()` | `indexes()` 里逆序填数组 | 反向迭代；**能不能 `.rev()` 取决于 trait（`DoubleEndedIterator`）** |
| `.enumerate()` | `find_pte` 的 `for (i, idx) in idxs.iter().enumerate()` | 同时拿下标和元素 |
| `.for_each()` | `clear_bss` | 遍历 + 副作用 |
| `Option::map` 的本质 | `frame_alloc` / `translate` | `Some(x) → Some(f(x))`，`None → None`；**把"可能失败"当普通值传递** |
| `.unwrap()` / `.expect()` | 到处 | `None`/`Err` 就 panic |
| `?` | `build.rs`（std 环境） | `Err` 提前 return |

**函数项也是值**：`.map(FrameTracker::new)` ≡ `.map(|ppn| FrameTracker::new(ppn))`。

---

## 4. unsafe 与底层

| 名字 | 出现位置 | 一句话 |
|---|---|---|
| `core::ptr::addr_of_mut!` | `heap_allocator.rs` | 拿 `static mut` 的地址**而不产生引用**（edition 2024 必需） |
| `core::slice::from_raw_parts_mut` | `get_pte_array` / `get_bytes_array` / `clear_bss` | `(raw ptr, len) → &mut [T]`，有效性自己保证 |
| `write_volatile` | `clear_bss` | 告诉编译器"这次写不能被优化掉" |
| `#[repr(C)]` | `PageTableEntry` | 字段布局按 C 规矩（不重排、不改） |
| `#[repr(align(4096))]` | ch3 的 `KernelStack`/`UserStack` | 强制对齐到页 |
| `asm!` / `global_asm!` | `sbi.rs`、`trap.S`、`switch.S`、`sfence.vma` | 内联汇编 / 把整个 .S 文件嵌进 crate |
| `unsafe extern "C" { safe fn f(); }` | `main.rs`、`trap.s` 的声明 | 声明"这个符号来自汇编/链接脚本"（edition 2024 新写法） |
| `#[unsafe(no_mangle)]` | `rust_main` | 不改名，让汇编/链接脚本能按名字找到（edition 2024 新写法） |

### ★ 一个必须内化的等式

```
PhysPageNum  →  &mut [T; 512]  （get_pte_array）
PhysPageNum  →  &mut [u8; 4096]（get_bytes_array）
```

这两行 `unsafe` 是**整个 ch4 的钥匙**：内核要按物理页号访问内存，靠的就是"内核地址空间里物理内存被**恒等映射**了"这个前提。
写任何 `unsafe` 时，**注释里必须写清它依赖的前提**（这是本项目注释规范的第 2 条）。

---

## 5. 语法糖与"看不见的机制"

| 名字 | 出现位置 | 一句话 |
|---|---|---|
| `use a::b::{self, C, D}` | `use core::fmt::{self, Debug, Formatter}` | 分组导入；花括号里的 `self` = **把模块自身也引进来**（之后可写 `fmt::Result`） |
| `super::` / `crate::` | `page_table.rs` 的 `use super::{...}` | 相对路径：`super` = 父模块，`crate` = 根 |
| 尾表达式 | 所有函数 | 函数体最后一个**无分号**表达式 = 返回值 |
| **auto-ref / auto-deref** | `FRAME_ALLOCATOR.exclusive_access()` | `x.m()` 会自动尝试 `&x`/`&mut x`/解引用后的类型 |
| `&&` vs `&`、`>>=` | `address.rs` 的 `indexes` | 逻辑与 vs 位与；复合赋值 |
| `0usize` / `as usize` | 到处 | 字面量类型后缀 = 显式标注类型 |
| `#[macro_export]` | `linker_symbol_address!` | 宏被导出到 **crate 根** → 子模块里要用 `crate::宏名!` |
| `#[allow(unused)]` | `map`/`unmap` 等 | 明知暂时没有调用者，先消警告 |

### ★ 三个"括号"（在 `address.rs::indexes` 里同时出现）

| 写法 | 是什么 |
|---|---|
| `[T; N]` | 数组类型（分号）；**`[v; N]` 是值**（重复 N 次的数组）—— 同形不同义 |
| `[T]` | 切片类型 |
| `(A, B)` | 元组类型（逗号） |

---

## 6. edition 2024 的新写法（本工程必须）

| 老写法（会报错/警告） | 新写法 |
|---|---|
| `static mut X` 直接取引用 | `core::ptr::addr_of_mut!(X)`（`static_mut_refs` 是 **deny-by-default**） |
| `extern "C" { fn f(); }` | `unsafe extern "C" { safe fn f(); }` |
| `#[no_mangle]` | `#[unsafe(no_mangle)]` |

⚠️ **注意**：硬错误存在时 **lint pass 不运行**，所以 `static_mut_refs` 这类错误会被前面的编译错误"遮住"，修完前面的才冒出来 —— 别以为是新引入的。

---

## 7. 陷阱合集（"我以为我知道"）

1. **`#[repr(C)]` 不是"防止编译器改结构名"** —— 名字压根不会被改。它管的是**字段布局**（不重排、不优化）。
   为什么需要：PTE 是**硬件按裸 8 字节直接读**的，布局必须稳定。
2. **`(1usize << 44) - 1` = `0xFFF_FFFF_FFFF`**（44 个 1 = 11 个 F），不是 `0x7FF_FFFF_FFFF`。**位运算的魔法数要手算核对。**
3. **`from_bits(x).unwrap()` 要求"所有位都有名字"**：`PTEFlags` 必须把 V/R/W/X/U/G/A/D 八个全定义，少一个就 panic。
4. **`&self` 却返回 `&mut`**（`find_pte`）能编译，是因为 `get_pte_array()` 返回 `&'static mut`（unsafe 制造的"撒谎生命周期"）→ `&self` 并不真保证不变性，这是别名可变引用的漏洞。
5. **`Option` 的 `Some` 不等于"成功"**：`translate()` 返回 `Some(pte)` 也可能 `!is_valid()`。**契约要自己从注释里读。**
6. **`use`-line 的注释 ≠ 有用注释**：路径本身已经自解释；有注释价值的是"为什么要引它"（如 `use StepByOne` 是为了调 `.step()`）。
7. **`///` 只作用于紧跟其后的那一个 item**。写块级说明必须用 `//`，否则文档注释会"挂"到下一个不相关的 item 上（`mm/mod.rs` 踩过）。
8. **`static mut` + 引用 = 未定义行为**，即使单核。要么 `addr_of_mut!`，要么包进 `UPSafeCell`。

---

## 8. 待学（ch5「进程」会出现）

| 名字 | 预计用途 |
|---|---|
| `Weak<T>` | 父进程指针（打破 `Arc` 循环） |
| `VecDeque` | 就绪队列（如果用到） |
| `Arc::clone` / `Arc::downgrade` / `upgrade` | 共享进程控制块 |
| `impl Trait for &T`（如 `Ord for &TCB`） | 用引用做比较排序 |
| `unsafe impl Send` | 跨核传递（ch8 才会认真用） |
| `Option::take()` | 从 `&mut Option<T>` 里"拿走"，把旧值变 `None` |
| `iter().position()` / `find()` | `find_task_by_pid` 之类 |
| `Rc` / `Arc` vs `Box` 的取舍 | 单一所有权用 `Box`，共享用 `Arc` |
