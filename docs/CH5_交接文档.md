# Odyssey · rCore 学习交接文档（ch4 完成 → ch5 待学）

> 用途：在新聊天中继续学 rCore ch5（进程）时，先让 Copilot 读取本文件 + `/memories/repo/odyssey-rcore-notes.md`，即可无缝衔接。
> 生成日期：2026-09-29

## 1. 当前状态一句话

rCore **ch4「地址空间」已学完、跑通、注释完工**（git `14483f6`「成功添加sv39页表机制，测试成功并编写注释」）。

内核已从 ch3 的「所有 app 挤在一片物理内存、各占固定槽位」升级为「**每个 app 一张 Sv39 页表、独立虚拟地址空间**」：启动时 `from_elf` 解析 ELF 的 program header 按页建立映射，内核地址空间 `KERNEL_SPACE` 恒等映射 `.text/.rodata/.data/.bss` + 物理内存 + MMIO，跳板 `trap.S` 同时映射在内核与每个 app 的页表上，`TrapContext` 搬进了 app 自己的地址空间。QEMU 里 6 个 app 交错跑完，`load_fault`/`store_fault` 的页错误被内核捕获并杀进程，末尾优雅关机。

ch4 的完整验收输出（可作为回归基线）：
```
.text [0x80200000, 0x80219000) .rodata [...] .data [...] .bss [0x80227000, 0x80538000)
mapping .text/.rodata/.data/.bss/physical memory/MMIO
remap_test passed!
init TASK_MANAGER
num_app = 6
power_3 [130000/300000] ...   (与 power_5 / power_7 输出交错)
Test power_5 OK! / Test power_7 OK! / Test power_3 OK! / Test sleep OK!
PageFault in application, bad addr = 0x0, bad instruction = 0x100e2, kernel killed it.
[kernel] Application exited with code 0
All applications completed!
```

### ch4 用时台账（2026-09-29 结算）

| 阶段 | 时长 | 日期 |
|---|---|---|
| 阶段1 跟着教程敲代码 | 11h | 09-12~13 (2h) / 09-17 (2h) / 09-20 (4h) / 09-22 (3h) |
| 阶段2 修 bug 并跑通 | 1.5h | 09-25 |
| 阶段3 注释 + 总结交付 | 2.5h | 09-29 |
| **合计** | **15h** | 6 次动手，日历跨度 18 天 |

### ch5 时长预估：**13~19h**（中位约 16h）

| 阶段 | ch4 实际 | ch5 预估 | 依据 |
|---|---|---|---|
| 阶段1 跟着敲代码 | 11h | 8~12h | ch5 **没有**"从零建一整个 mm 子系统"这种大头，主要是改 `task/` + 接线；扣分项是多了一批小 user app（initproc / user_shell / forktest 系列） |
| 阶段2 修 bug | 1.5h | **2~4h** | ch4 的 4 个 bug 几乎全是"工程问题"（好查）；ch5 的 fork/exec 是**语义 bug**（症状诡异）。但编译错误会少得多，且已有经验 |
| 阶段3 注释 + 总结 | 2.5h | 3h | 要 review 的文件更多（`task/*` 大改 + 新增文件） |
| **合计** | **15h** | **13~19h** | 约 ch4 的 0.9~1.3 倍 |

> **校准说明（重要，影响以后所有估时）**
> 09-20 曾估「ch4 剩余跑通 15~25h」，实际此后只花了 **4.5h**（09-22 的 3h + 09-25 的 1.5h）→ **对"剩余调试量"的估算偏高约 3~5 倍**。
> 但对"整章总量"的估算偏高约 2 倍（09-20 时点整章实际约 15h，当时的估法是"已完成 + 15~25h" ≈ 30h+）。
> 两种口径收敛：官方教程给的 ch5 估时 **25~40h 打对折 ≈ 13~20h**，与上表的阶段拆解法 **13~19h** 吻合 → 取 **13~19h**。
> **以后给用户估时的口诀：先按"每天能推进的量"拆，再整体除以 2。**

### 预计最吃时间的地方（ch5 卡点预警）

1. **fork 的地址空间逐页拷贝** —— 容易漏掉"改子进程 TrapContext 的 a0"这一步。
2. **exec 后必须等回到用户态再让新 satp 生效** —— 逻辑最绕。
3. **`Arc`/`Weak` 循环引用** —— 症状是"内存慢慢少"，不报错。
4. **idle 控制流** —— 要从 ch4 的"数组 + 下标"思维切换过来。
5. **user_shell 读输入** —— 读不到时要"忙等 + 让出 CPU"，不能死循环。

### 执行方式（2026-09-29 确认，沿用 ch4 的三段式）

1. **用户自己读 rCore + 跟着敲代码**，Copilot 只回答概念问题、不代写代码（用户明确要求时例外）；
2. 敲完 → **一起测试、修 bug**；
3. 最后 → **代码框架地图 + 逐文件 review 注释**。

## 2. 环境与命令（ch4 之后有变化，务必看）

- 工具链：`Odyssey/rust-toolchain.toml` 锁 **`nightly-2025-02-18`**，**edition 2024**，`#![feature(alloc_error_handler)]`
- 依赖：`riscv = { git, rev = "11d43cf7cccb3b62a3caaf3e07a1db7449588f9a" }`（0.6.0，**勿换** crates.io 0.16）；`sbi-rt 0.0.2`；`lazy_static 1.4`；`buddy_system_allocator 0.6`；`bitflags 1.2.1`；**`xmas-elf 0.7.0`（ch4 新增）**
- 编译（两步，顺序不能反）：
  1. `cd user && make build`
  2. `cd os && cargo build`
- 运行：`cd os && make run`

### ⚠️ ch4 带来的三处命令/脚本变化

1. **`os/build.rs` 内嵌的是 ELF，不再是 objcopy 出来的 `.bin`**（ch4 的 `from_elf` 要用 xmas-elf 解析 program header）。
   `user/Makefile` 里那个 objcopy 步骤对内核已无意义（保留无害，可清理）。
2. **`user/build.py` 的「逐 app 换基址」从 ch4 起已成空转**：它替换的是 `0x80400000`，而 `user/src/linker.ld` 现在写的是 `0x10000`，替换命不中任何一行。
   现象上无害（各 app 都链在 0x10000，正是 ch4 想要的），但可以清理成一次 `cargo build --release`。
3. **`.cargo/config.toml` 里有 `-Clink-arg=-Tsrc/linker.ld`**，内核链接脚本由它指定。

### ⚠️ 两条必须记住的操作纪律

- **改完 app 的 ELF，一定要 `cd os && cargo clean`**。否则 `link_app.S` 里的 `.incbin` 内容陈旧，而 `cargo build` 只花 0.3s 就"成功"了 —— 这是最容易骗到自己的假成功。
- **`make run`（`-nographic`）的输出经管道重定向会丢失**。可靠抓法：
  `qemu-system-riscv64 -machine virt -display none -bios default -kernel <elf> -serial file:/tmp/x.log`

## 3. 代码地图（ch4）

```
Odyssey/
├─ rust-toolchain.toml                  # 锁 nightly-2025-02-18
├─ docs/CH4_交接文档.md / CH5_交接文档.md # 本文件
├─ Docs/开发日志.md + Rust生词本.md
├─ os/                                  # 内核（S 态）
│  ├─ .cargo/config.toml                # -Clink-arg=-Tsrc/linker.ld
│  ├─ build.rs                          # 扫 user/src/bin/*.rs → 生成 src/link_app.S（.incbin 各 app 的【ELF】）
│  ├─ Makefile                          # make run
│  └─ src/
│     ├─ main.rs                        # clear_bss → mm::init → mm::remap_test → trap::init →
│     │                                 #   enable_timer_interrupt → set_next_trigger → task::run_first_task
│     ├─ config.rs                      # PAGE_SIZE/PAGE_SIZE_BITS/MEMORY_END=0x8800_0000/
│     │                                 #   TRAMPOLINE=usize::MAX-PAGE_SIZE+1/TRAP_CONTEXT=TRAMPOLINE-PAGE_SIZE/
│     │                                 #   MMIO/CLOCK_FREQ=10_000_000/kernel_stack_position(app_id)
│     ├─ linker.ld                      # BASE_ADDRESS=0x80200000；.text.entry → ALIGN(4K) → strampoline
│     │                                 #   → *(.text.trampoline) → ALIGN(4K) → .text；sbss_with_stack 在 .bss.stack 前
│     ├─ entry.asm / lang_items.rs / console.rs / sbi.rs / timer.rs
│     ├─ sync/{mod,up}.rs               # UPSafeCell（RefCell + unsafe impl Sync）/ exclusive_access()
│     ├─ mm/
│     │  ├─ mod.rs                      # 【门面】5 个私有子模块 + 精选 re-export + mm::init()
│     │  ├─ address.rs                  # 4 个 newtype（PhysAddr/VirtAddr/PhysPageNum/VirtPageNum）
│     │  │                              #   + usize↔newtype 转换矩阵 + StepByOne + SimpleRange/VPNRange
│     │  ├─ heap_allocator.rs           # #[global_allocator] LockedHeap + HEAP_SPACE（KERNEL_HEAP_SIZE）
│     │  ├─ frame_allocator.rs          # FrameTracker(RAII) + StackFrameAllocator（回收栈, LIFO）+ frame_alloc/dealloc
│     │  ├─ page_table.rs               # PTEFlags / PageTableEntry / PageTable（3 级走表 map·unmap·translate·token）
│     │  │                              #   + translated_byte_buffer（用户指针 → 内核切片）
│     │  └─ memory_set.rs               # MemorySet{page_table, areas} / MapArea{map_type, map_perm, data_frames}
│     │                                 #   / MapType{Identical, Framed} / MapPermission / from_elf / KERNEL_SPACE
│     ├─ trap/
│     │  ├─ mod.rs                      # trap::init（stvec=__alltraps）/ trap_return() / trap_handler
│     │  ├─ context.rs                  # TrapContext（36 槽：x[32] + sstatus + sepc + 【kernel_satp + kernel_sp + trap_handler】）
│     │  └─ trap.S                      # 【跳板】__alltraps / __restore（.text.trampoline 段，纯 ASCII）
│     ├─ task/
│     │  ├─ mod.rs                      # TaskManager（tasks + current）/ TASK_MANAGER / run_first_task / run_next_task
│     │  ├─ task.rs                     # TaskControlBlock{task_status, task_cx, memory_set, trap_cx_ppn, base_size,
│     │  │                              #   heap_bottom, program_brk} / get_trap_cx / get_user_token / change_program_brk
│     │  ├─ context.rs                  # TaskContext{ra, sp, s[12]} / goto_trap_return / goto_restore
│     │  ├─ switch.rs / switch.S        # __switch：保存/恢复 ra,sp,s0~s11（纯 ASCII）
│     ├─ syscall/{mod,fs,process}.rs    # 分发 write(64)/exit(93)/yield(124)/get_time(169)
│     └─ (batch.rs / loader.rs 已是 ch3 残留，仍被 mod 声明但不参与运行 → 可删)
└─ user/                                # 应用库（U 态, no_std）
   ├─ build.py                          # ⚠️ ch4 后已成空转（见 §2）
   ├─ Makefile
   └─ src/
      ├─ lib.rs / syscall.rs / console.rs / lang_items.rs / linker.ld
      │                                 # linker.ld：BASE_ADDRESS=0x10000，每段前 . = ALIGN(4K)
      └─ bin/{00power_3, 01power_5, 02power_7, 03sleep, 04load_fault, 05store_fault}.rs
      └─ (pending_bins/sbrk_test.rs —— 等 sbrk 接通后移回 bin/)
```

### ch4 的两条控制流通道（务必分清）

| 通道 | 代码 | 何时走 | 存什么 |
|---|---|---|---|
| **进出用户态** | `trap.S` 的 `__alltraps` / `__restore` | 用户态 ↔ 内核态 | **全部**寄存器（36 槽 TrapContext） |
| **内核内换任务** | `switch.S` 的 `__switch` | 内核态内 | 只存 **callee-saved**（TaskContext：ra/sp/s0~s11） |

`__alltraps` 里的三件关键事：① 换栈（`csrrw sp, sscratch, sp`）② `csrw satp, kernel_satp` + `sfence.vma`（换内核页表）③ `jr trap_handler`。
`__restore` 里：`csrw satp, a1`（换回用户页表）+ `sfence.vma` → 恢复全部寄存器 → `sret`。
**两者都必须在跳板里**，因为写 `satp` 的那一瞬间，PC 必须落在"两张页表都映射到同一物理页"的代码上。

## 4. ch4 经验教训（★ = 最值钱）

### A. 编译 / 接线类

1. **"文件存在 ≠ 文件被编译"**：Rust 模块树由 `mod` 声明搭出来。`mm/heap_allocator.rs` 在 `main.rs` 写 `mod mm;` + `mm/mod.rs` 写 `mod heap_allocator;` 之前是个**孤儿文件**，症状是 `no global memory allocator found but one is required`（编译器根本没看见 `#[global_allocator]`）。
2. **可见性链要逐段看**：私有 `mod` 只对"自身 + 后代"可见，**父模块看不到**。要 `mm::heap_test()` 必须 `pub use` re-export。
3. **trait 方法调用需要 trait 在作用域内**：`vpn.step()` 是 `StepByOne::step(&mut vpn)` 的糖 → 没有 `use StepByOne` 就报 `E0599 ... help: traits need to be in scope`。这是"import 了一个代码里看不见的名字"的典型。
4. **edition 2024 的 `static_mut_refs` 是 deny-by-default**：`static mut HEAP_SPACE.as_ptr()` 必须换成 `core::ptr::addr_of_mut!`。
5. ★ **硬错误存在时 lint pass 不运行** → 有些错误会被前面的错误"遮住"，修完才冒出来。**所以别指望"一次修完"，从最早一条往下修。**

### B. 运行 / 语义类（最隐蔽，"看着对却跑飞"）

6. ★★ **`user/src/linker.ld` 段间缺 `. = ALIGN(4K);`** —— ch4 最贵的一课。
   - 现象：`Panicked at page_table.rs:114  vpn VPN:0x11 is mapped before mapping`
   - 根因：`.text` 结束于 `0x111d8`，`.rodata` 就从 `0x111d8` 开始 → **两个 program header 落在同一页** → `from_elf` 按页映射时同一个 VPN 被映射两次。
   - 教训：**页表以"页"为单位**。任何"按段/按字节"的布局，都要先问一句"这两段的页号会不会撞"。
   - 顺带：这次是 `map()` 里那句 `assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping")` 把 bug 抓出来的 —— **断言是免费的调试器**。
7. **`satp` 写后必须 `sfence.vma`**：TLB 不会因为写 `satp` 自动失效，漏了就是"偶尔错一次"的玄学 bug。
8. **内核不能直接解引用用户指针**：用户页带 U 位，S 态访问会触发页错误；而且用户指针的合法性完全不可信 → 一律走 `translated_str` / `translated_byte_buffer`。
9. **`PageTable::from_token()` 的 `frames: Vec::new()` 是故意的**：同一个 `PageTable` 类型有两种所有权语义 —— `new()` **拥有**页表页（frames 里有 FrameTracker，drop 时归还），`from_token()` 只是"借来看"（不拥有任何页）。漏掉这个区别，就会误以为"忘了写 frame_alloc"，一旦"顺手补上"就会拆掉别人的页表。
10. **中间级 PTE 只能带 `V`**（R/W/X 必须全 0），否则硬件会把它当叶子（大页）当场翻译，**跳过下面 512 项**。
11. **"添加/删除映射" = 往 8 字节 PTE 里写整数**，因为页表是**内核与 MMU 共享的数据结构**；改完要刷 TLB → 严格说是"改 PTE + 刷 TLB"两步。
12. **职责分离**：`PageTable` 拥有**页表页**；**数据页由 `MapArea::data_frames` 拥有**。所以 `unmap` 清 PTE **不会**回收数据页 —— 回收靠持有 FrameTracker 的一方 drop。

### C. 构建 / 工程类

13. **`os/build.rs` 必须内嵌 ELF**（不是 objcopy 的 `.bin`）。报 `Did not find ELF magic number` 就是这里。
14. ★ **从 Windows 拷文件进 WSL 会带出 `*:Zone.Identifier` 元数据文件** —— 在 ext4 里它们是**真的文件**。7 个垃圾文件让 `build.py` / `build.rs` 把每个 app 数成两个（`_num_app = 14`），症状是"应用数量莫名翻倍"。
15. **user 编不过 → 缺 ELF → os 报 `Could not find incbin file`**。见到 incbin 报错，**先回 user 看编译错误**。（本次真凶：`sbrk_test.rs` 依赖尚未实现的 `user_lib::sbrk`。）
16. **cargo 不跟踪 `linker.ld` 内容、也不跟踪 `.incbin` 的中间产物** → 该 `cargo clean` 就 clean。
17. `.s` / `.S` 里**不能有中文/非 ASCII**（rustc 会 ICE）。

### D. 常量 / 细节

18. **`CLOCK_FREQ = 10_000_000`**（本机 QEMU virt 的 mtime 实测 10 MHz，串口日志开头会打印 `aclint-mtimer @ 10000000Hz` 佐证）。填错会同时影响时间片长度和 `sys_get_time`。
19. **`MEMORY_END = 0x8800_0000`**（原为 `0x8080_0000`，那样帧分配器只剩约 6MB 可用）。
20. `TRAMPOLINE = usize::MAX - PAGE_SIZE + 1`、`TRAP_CONTEXT = TRAMPOLINE - PAGE_SIZE` —— 这两个魔法数在 ch4 到处出现，看到它们要立刻反应"这是虚拟地址空间最顶上那两个页"。
21. `kernel_stack_position(app_id)` 用 `app_id` 错开内核栈（ch5 会改成按需动态分配）。
22. `PTEFlags` 的 **8 个位必须全部定义**，否则 `PTEFlags::from_bits(bits).unwrap()` 会 panic（`from_bits` 只接受"所有位都有名字"的值）。

### E. 方法论（怎么修 bug）

23. ★ **从最早的错误往下修**：106 个编译错误，实际只有约 10~15 个根因，后面的多数是前面的影子。
24. ★★ **先怀疑"人为约定"，再怀疑"算法"**：从 M1 到跑通只剩 4 个问题，**没有一个是逻辑 bug** —— 全是"缺 .bin / 垃圾文件 / 内嵌格式 / 段对齐"。内核 bug 的第一嫌疑人是**链接脚本、构建脚本、常量**，不是代码逻辑。
25. **读完整条报错**：`help: traits need to be in scope`、`could not find config in the crate root`、`inaccessible` —— 编译器常把答案直接写在最后一行。
26. **现象要能隔离复现**：`-serial file:` 抓日志、`cargo clean` 排除陈旧产物、用符号表确认内存布局，比干看代码快得多。
27. ★ **注释是第二次学习**：写注释的过程中才发现 `#[repr(C)] // 防止编译器修改结构名` 写错了（`repr(C)` 管的是**字段布局**，和名字无关）、`(1usize<<44)-1` 的值算错了（是 `0xFFF_FFFF_FFFF`，不是 `0x7FF_FFFF_FFFF`）、"新建页表项"和"新建页表页"分不清。**写不出那一句话，就说明还没懂。**

### F. 注释规范（ch4 期间定下的，后续沿用）

1. 注释回答「**为什么 / 契约 / 陷阱**」，不复述代码。
2. 每个**魔法数**、每条**不变量**、每处 **unsafe 承诺**都必须有注释。
3. **全项目一种语言（中文）**；`use` 行默认不写注释。
4. 四个"必写位置"：① 文件头 `//!` ② 每个 pub 类型的 `///` ③ 魔法数/位运算/unsafe/断言的行内注释 ④ 有隐式语义处（`Drop`、`unsafe impl`）。
5. 注释水平四级阶梯：① 复述代码 → ② 解释做法 → ③ 解释意图 → ④ **写清契约与代价**（隐含前提、unsafe 保证、代价、调用方）。
   `mm/address.rs` 是 ④ 级模板；`mm/page_table.rs` 也已完工。
6. 格式铁律：`///` **只作用于紧跟其后的那一个 item**；块级说明必须用 `//`。

## 5. ch4 遗留待办（建议开学 ch5 前先清）

| # | 事项 | 位置 | 说明 |
|---|---|---|---|
| 1 | **接通 `sys_sbrk`**（4 处）⚠️ **用户自己做，Copilot 勿代写**（2026-09-29 明确"先别管让我自己来"） | `syscall/mod.rs`（`SYSCALL_SBRK = 214`）、`syscall/process.rs`（`sys_sbrk`）、`task/mod.rs`（`change_current_program_brk` 包装）、`user/src/syscall.rs`（`sbrk`） | 内核侧的 `TaskControlBlock::change_program_brk` / `MemorySet::{shrink_to, append_to}` / `MapArea::{shrink_to, append_to}` **都已写好**（带 `// [待学: sys_sbrk]` 标签），只差"接线"。接完把 `user/pending_bins/sbrk_test.rs` 移回 `user/src/bin/` |
| 2 | 删 ch3 残留 | `os/src/batch.rs`、`os/src/loader.rs`（+ `main.rs`/`mod.rs` 里的 `mod` 声明） | 编译着但不参与运行 |
| 3 | 清 `config.rs` 无用常量 | `MAX_APP_NUM` / `APP_BASE_ADDRESS` / `APP_SIZE_LIMIT` | ch4 起不再使用 |
| 4 | 清理 `user/build.py` | 见 §2.2 | 换基址已成空转 |
| 5 | 注释收尾 | `trap/*`、`task/*`、`memory_set.rs`、`syscall/*`、user 侧 | 按 `page_table.rs → mm/mod.rs → memory_set.rs → trap/* → task/task.rs` 的顺序推进；`memory_set.rs` 现状 50/449 |
| 6 | 注释体检命令留给下次 | — | `grep -cE '^[[:space:]]*//'` 数整行注释，注意**尾注释不计入**，会低估 `sbi.rs`/`console.rs` |

## 6. rCore ch5 预习提纲（下次主题：**进程**）

官方章节结构：引言 → 进程概念及重要系统调用 → 进程管理的核心数据结构 → 进程管理机制的设计实现 → 进程调度（理论）。

**一句话主线**：ch4 里「一个 app = 一个地址空间 + 一个 TCB」，是**静态、一次性**的；ch5 要让它**动态**起来 —— 运行中的进程能**复制自己（fork）**、**改头换面（exec）**、**等别人结束（waitpid）**、**被别人等**，还要有 **idle 控制流**兜底。

### 预计会遇到的概念

1. **进程 = 程序 + 执行状态 + 资源**：`pid`、父子关系、生命周期（Ready / Running / Zombie / Exited）。
2. **三个关键系统调用**
   - `fork()`：**复制**当前进程（地址空间逐页拷贝 + 复制 TrapContext）→ 子进程返回 0，父进程返回子进程 pid；**同一个函数返回两次**。
   - `exec(path, args)`：**替换**当前进程的地址空间（重新 `from_elf`），回到用户态执行新程序；**不返回**（除非失败）。
   - `waitpid(pid, exit_code_ptr)`：父进程阻塞等子进程结束，并回收它的资源。
   - 配套：`exit(exit_code)`、`getpid()`。
3. **两个新数据结构**
   - **PID 分配器**：回收式分配（用过的 pid 要能再用）。
   - **内核栈改为动态分配**：ch4 的 `KERNEL_STACK` 是静态数组 + `kernel_stack_position(app_id)` 定位；ch5 要用 `frame_alloc` 按需分配（因为有 `fork`，进程数不再固定）。
4. **进程控制块结构重组**
   - 外层（调度器要用、要能被直接改）：`task_status`、`task_cx`
   - 内层（要能**父子共享**）：`pid`、`kernel_stack`、`memory_set`、`trap_cx_ppn`、`heap_bottom`、`program_brk`
   - 预计形状：`TaskControlBlock { parent: Option<Weak<...>>, inner: Arc<UPSafeCell<TCBInner>>, task_cx, task_status }`
   - **用 `Weak` 存父进程**：否则父子互相 `Arc` 引用 → 引用计数永不归零 → 泄漏。这是 `Arc/Weak` 第一次真正派上用场。
5. **任务管理器与处理器管理结构**
   - `TaskManager`：`Vec<Option<Arc<TCB>>>` + `find_task_by_pid` + 按 pid 增删
   - `Processor`：`current: Option<Arc<TCB>>` + **`idle_task_cx`** —— 用"idle 控制流"代替 ch4 的"数组 + current 下标"，没有就绪任务时切到 idle 任务空转
6. **`fork` 的实施细节**（预计最绕）
   - `MemorySet::from_existed_user`：逐页 `frame_alloc` + 拷贝数据 + 用同样的 mapping 建新页表
   - 复制后要**拿到新的 `trap_cx` 并改 `a0`**（子进程的返回值 0）
   - 新进程要挂进 `TaskManager`，并准备一个"从这里开始跑"的 `TaskContext`
7. **`exec` 的实施细节**：替换 `memory_set` → 更新 `trap_cx_ppn` → 重建 TrapContext → **必须等回到用户态之后再让新 satp 生效**（因为换 satp 时 PC 必须在跳板上）。
8. **`sys_read` + 控制台输入**：`sbi.rs` 已有 `console_getchar()`；shell 需要"读一行"的能力。读不到时预计要用 `suspend_current_and_run_next()` 让出 CPU（**忙等 + 让出**），而不是死循环。
9. **应用加载方式可能变化**：ch5 的 user 程序要能**按名字**被 `exec`（`initproc` 拉起 `user_shell`，shell 再 `exec` 别的 app）→ 预计 `os/build.rs` 要多导出一张"应用名表"，`loader.rs` 要多一个 `get_app_data_by_name`。同时 `user/build.py` 的换基址逻辑应被彻底清掉（所有 app 统一链在 `0x10000`）。
10. **调度理论**（4scheduling）：FCFS / SJF / STCF / 时间片轮转 / 多级反馈队列 / 公平份额（stride）/ 实时（RMS, EDF）。ch5 的**代码**仍是 ch3 那套时间片轮转，但这章会系统讲清"为什么是它"。

### 预计新增 / 大改的文件

- `os/src/task/{mod.rs, task.rs}` —— **改动最大**（TCB 拆分、Processor/idle、pid、fork/exec/exit/wait 的骨架）
- `os/src/task/{id.rs（或 pid.rs 新增）, processor.rs（可能新增）}`
- `os/src/mm/memory_set.rs` —— 加 `from_existed_user`
- `os/src/syscall/{mod.rs, process.rs, fs.rs}` —— 加 `fork/exec/waitpid/getpid/read`
- `os/src/loader.rs` —— 按名加载（ch4 的残留文件在这里"复活"）
- `os/build.rs` —— 导出应用名表
- `user/src/lib.rs`、`user/src/syscall.rs` —— 封装 fork/exec/waitpid/getpid/read
- `user/src/bin/{initproc.rs, user_shell.rs, forktest*.rs, exit.rs, wait.rs, ...}` —— 一堆新 app
- 清掉：`user/build.py` 的换基址、`os/src/batch.rs` 与旧 `loader.rs`

### 建议动手前先巩固 ch4 这几点

- `TrapContext` 的 36 个槽分别是什么，尤其**最后三个**（`kernel_satp` / `kernel_sp` / `trap_handler`）为什么必须存在；
- `trap_return()` 的推导：`__restore - __alltraps + TRAMPOLINE`；
- `TaskContext::goto_trap_return(kernel_stack_top)` 与 `TaskContext::goto_restore` 的差别；
- `PageTable` 的两种所有权语义（`new` vs `from_token`）；
- `MemorySet::from_elf` 里"哪些段是 Framed、哪些是 Identical、`MapPermission` 怎么定"；
- `TASK_MANAGER` / `KERNEL_SPACE` 这类 `lazy_static! + UPSafeCell` 全局量的访问姿势（`exclusive_access()` 而非 `borrow_mut()`）。

## 7. 新会话开场白（可直接复制）

### 7.1 完整版（推荐，第一次开新会话用）

```
读取 Odyssey/docs/CH5_交接文档.md 和 repo memory 里的 odyssey-rcore-notes.md、
odyssey-study-log.md、odyssey-rust-vocab.md，我们开始学 rCore ch5（进程）。

【现状】
- ch4「地址空间」已完成、跑通、注释完工，git 14483f6，工作区干净
- os / user 都在 WSL /root/Odyssey
- ch4 遗留待办见交接文档 §5 —— 其中 sys_sbrk 由我自己实现，你不要代写

【本次做法：沿用 ch4 的三段式】
1. 我自己读 rCore 教程、自己跟着敲代码。你只回答我提出的概念问题，
   不要主动给代码、不要替我改文件；（我明确说"帮我写"时才动手）
2. 代码全敲完后，我们一起测试、修 bug；
3. 最后你输出代码框架地图（各文件目的）+ 逐文件 review 我的注释。

【注释规范】见交接文档 §4.F：中文、写"为什么/契约/陷阱"、四级阶梯、
/// 只作用于紧跟其后的那一个 item。

【现在】从 ch5 §5.1「进程概念及重要系统调用」开始，我先读、先敲。
```

### 7.2 短版（顺手用）

> 读取 `Odyssey/docs/CH5_交接文档.md` 和 repo memory 里的 `odyssey-rcore-notes.md`，我们继续学 rCore ch5（进程）。ch4 已完成并跑通（git `14483f6`），老规矩：我自己读教程跟着敲代码，你只答概念问题不代写；敲完再一起调 bug，最后给我框架地图 + 指点注释。`sys_sbrk` 我自己做。

### 7.3 换章节时的一句话模板

> 读取 `Odyssey/docs/CH<n+1>_交接文档.md` 和 repo memory 里的 `odyssey-rcore-notes.md`，我们继续学 rCore ch<n>（<章节名>）。上一章已完成并跑通（git `<hash>`），老规矩三段式。
