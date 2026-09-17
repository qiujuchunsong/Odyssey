# Odyssey · rCore 学习交接文档（ch3 完成 → ch4 待学）

> 用途：在新聊天中继续学 rCore ch4 时，先让 Copilot 读取本文件 + `/memories/repo/odyssey-rcore-notes.md`，即可无缝衔接。
> 生成日期：2026-09-11

## 1. 当前状态一句话
rCore **ch3「多道程序与分时多任务」已学完并跑通**：`os` 内核 + `user` 用户库可编译；4 个 app（`00power_3` / `01power_5` / `02power_7` / `03sleep`）内嵌进内核；启动时一次性加载全部 app，每个任务有自己的内核栈/用户栈/TCB，内核用 `__switch` 切换任务，**时钟中断每 10ms 抢占一次**、`sys_yield` 可主动让出、`sys_get_time` 可读时间。QEMU 里可见三个计算 app 的输出**交错**、`Test sleep OK!`、以及应用正常退出（`Application exited with code 0`）。

## 2. 环境与命令（重要）
- 工具链：`Odyssey/rust-toolchain.toml` 锁 **`nightly-2025-02-18`**（stable 编不过 `riscv 0.6.0` 的 `asm_const`）
- 依赖版本：`riscv = { git, rev = "11d43cf..." }`（0.6.0，**勿用** crates.io 新版）；`sbi-rt 0.0.2`；`lazy_static`
- **编译顺序（关键）**：
  1. `cd user && make build` —— 内部跑 `python3 build.py`：**逐 app 改 `linker.ld` 基址 → 单独 `cargo build --bin X --release` → 还原**，再 objcopy 出 `.bin`
  2. `cd os && cargo build` —— `build.rs` 扫 `user/src/bin/*.rs` 生成 `src/link_app.S`，把 `.bin` `.incbin` 进内核
- 运行：`cd os && make run`（qemu-system-riscv64，`-machine virt -nographic -bios default`）
- `user/Makefile` 两个要点：`elf:` 目标 = `@python3 build.py`（recipe 必须 **Tab**）；`OBJCOPY := /root/.cargo/bin/rust-objcopy --binary-architecture=riscv64`（**绝对路径**）
- 从 VS Code PowerShell 调 WSL 的坑：`$` 被 PowerShell 展开、内层双引号被吞、`<`/`>` 被当重定向报错 → 尽量**单引号包整段 bash**、避免变量、用 `-serial file:` / `tee` 代替重定向

## 3. 代码地图（ch3）
```
Odyssey/
├─ rust-toolchain.toml            # 锁 nightly-2025-02-18
├─ docs/CH4_交接文档.md           # 本文件
├─ Docs/开发日志.md
├─ os/                            # 内核(S态)
│  ├─ build.rs                    # 扫 user/src/bin/*.rs → 生成 src/link_app.S（.incbin 各 .bin）
│  ├─ Makefile                    # make run
│  └─ src/
│     ├─ main.rs                  # clear_bss → trap::init → loader::load_apps → trap::enable_timer_interrupt → timer::set_next_trigger → task::run_first_task
│     ├─ config.rs                # USER/KERNEL_STACK_SIZE, MAX_APP_NUM, APP_BASE_ADDRESS, APP_SIZE_LIMIT, CLOCK_FREQ=10_000_000
│     ├─ loader.rs                # KERNEL_STACK/USER_STACK 数组; load_apps(); init_app_cx(); push_context()
│     ├─ timer.rs                 # get_time()(mtime) / get_time_us() / set_next_trigger()(TICKS_PER_SEC=100 → 10ms)
│     ├─ sbi.rs                   # console_putchar / shutdown / set_timer(sbi_rt::set_timer)
│     ├─ console.rs / lang_items.rs / entry.asm / linker.ld
│     ├─ sync/{mod,up}.rs         # UPSafeCell（RefCell + unsafe impl Sync），exclusive_access()
│     ├─ task/
│     │  ├─ mod.rs                # TaskManager / TaskManagerInner / TASK_MANAGER(lazy_static) / 调度: run_first_task, run_next_task, find_next_task, suspend/exit_current_and_run_next
│     │  ├─ task.rs               # TaskControlBlock{task_status, task_cx} / TaskStatus{UnInit,Ready,Running,Exited}
│     │  ├─ context.rs            # TaskContext{ra,sp,s[12]} / zero_init / goto_restore
│     │  ├─ switch.rs             # global_asm!(switch.S) + 声明 __switch
│     │  └─ switch.S              # __switch：保存/恢复 ra,sp,s0~s11（纯 ASCII！）
│     ├─ trap/
│     │  ├─ mod.rs                # stvec=__alltraps; trap_handler: UserEnvCall / PageFault / IllegalInstruction / SupervisorTimer
│     │  ├─ context.rs            # TrapContext{ x[32], sstatus, sepc }（34 槽）; app_init_context
│     │  └─ trap.s                # __alltraps / __restore（纯 ASCII！）
│     └─ syscall/{mod,fs,process}.rs  # 分发 write(64)/exit(93)/yield(124)/get_time(169)
│     └─ (batch.rs 已弃用，不参与编译)
└─ user/                          # 应用库(U态, no_std)
   ├─ build.py                    # 逐 app 改 linker.ld 基址（KEY）
   ├─ Makefile                    # elf: python3 build.py; binary: objcopy
   └─ src/
      ├─ lib.rs / syscall.rs / console.rs / lang_items.rs / linker.ld
      └─ bin/{00power_3,01power_5,02power_7,03sleep}.rs
```

## 4. 已踩过的坑（新会话别重踩）

### A. 编译 / 接线类
1. **user 编译错误 → 缺 `.bin` → os 报 `Could not find incbin file`**。`build.rs` 是**按 `user/src/bin` 的文件名**生成 `.incbin` 的，`user` 一旦编不过，新的 `.bin` 就不会生成。**见到 incbin 报错，先回 user 看编译错误。**
2. **Rust 模块树由 `mod` 声明搭建**：`config.rs` 和 loader 同目录，但 `main.rs` 里没写 `mod config;` → `use crate::config::*` 报 “could not find config in the crate root”。**同目录 ≠ 模块。**
3. **`extern "C"` 块只能放“没有函数体的声明”**：写成 `pub unsafe fn __switch() { ... };` → `no __switch in task::switch`。正确写法：参数写在括号里、以分号结尾。
4. **`UPSafeCell` 要用自己的访问器 `exclusive_access()`**。误用 `.borrow_mut()` 时会被 `use core::borrow::BorrowMut;` 兜底到 `BorrowMut<Self> for Self`，返回“包装器自身”的 `&mut`，于是报 `no field ... on &mut UPSafeCell<...>`。另外字段是 `tasks`（复数）；全局静态名是 `TASK_MANAGER`（`Task_Manager`/`Task_Maneger` 都拼错）。
5. 函数名不一致：`loader.rs` 里是 `load_app`，`main.rs` 调 `load_apps`。
6. **系统调用分发漏项**：`syscall()` 只匹配了 WRITE/EXIT，漏了 YIELD/GET_TIME → 运行到 `yield` 就 panic `Unsupported syscall_id`。编译期的 `never used` 警告（`sys_yield`/`sys_get_time`/`SYSCALL_YIELD`）正是线索。
7. API 版本：本版 riscv 里是 `sie::set_stimer()`（不是 `set_timer`）；`Interrupt` 要从 `scause` 导入；`sbi.rs` 里别引用不存在的 `sbi_call`，直接用 `sbi_rt::set_timer(u64)`。

### B. 运行 / 汇编语义类（最容易“看着对却跑飞”）
8. **`trap.s` 的 `__restore` 不能保留 ch2 的 `mv sp, a0`** —— 这是 ch3 最隐蔽的坑。
   - ch2：`__restore(cx)` 是普通函数调用，`a0 = cx`，所以要 `mv sp, a0`。
   - ch3：任务**首次**经 `__switch → ret 到 __restore` 进入，此时 `a0` 是 `__switch` 的第一个参数（不是 TrapContext），而 **`sp` 已由 `__switch` 设好**（= TrapContext 地址）。`mv sp, a0` 会把 sp 劫持到 boot 栈 → 读垃圾 sstatus/sepc、写坏内存（含 `RefCell` 借用标志）→ 表现为 `BorrowMutError`（`src/sync/up.rs:29`）。
   - 教训：**同一个 `__restore`，ch2/ch3 的“进入约定”不同**；ch3 直接从 `ld ...(sp)` 开始。
9. **app 必须在它“被加载的地址”上链接** —— ch3 第二大坑。
   - 内核把第 i 个 app 加载到 `APP_BASE_ADDRESS + i*APP_SIZE_LIMIT`（本例 `0x80400000 + i*0x20000`）。
   - 但 `cargo build --release` 会按 `linker.ld` 里固定的 `BASE_ADDRESS = 0x80400000` 把**所有 app 链在同一处**；app 内对字符串等数据的引用是**绝对地址**，于是 app1/2/3 读到的是 **app0 的数据** → `Utf8Error`（`fs.rs` 的 `from_utf8().unwrap()`）、一堆单空格乱码、最后 `InstructionFault`。
   - 正解：`user/build.py` 为每个 app 临时改 `linker.ld` 基址再单独编译。
10. **调试经验**：把现象「隔离复现」（复制到 `/tmp` 插桩，不动原工程）、用符号表（`rust-nm`）确认内存布局、用「关掉抢占 / 打日志 dump (ptr,len,bytes) / 打印调度轨迹」来二分定位，比干看代码快得多。

### C. 构建 / 工程类
11. **`make` 的 recipe 必须 Tab**：`elf:` 下写 4 个空格 → `missing separator`。
12. **`make` 子 shell 里 `rust-objcopy` 解析失败**（`Permission denied`）→ `OBJCOPY` 写成绝对路径 `/root/.cargo/bin/rust-objcopy ...`。
13. **cargo 不跟踪 `linker.ld` 内容 / `.incbin` 的中间产物**：
    - 改 app 基址后需 `cd user && cargo clean && make build`（否则 `cargo build --bin X` 认为“Fresh”不重新链接）；
    - 改了 `.bin` 后 os 未必重新汇编嵌入 → `cd os && cargo clean`（或 `touch os/src/main.rs`）。
14. `.s`/`.S` 里**不能有中文/非 ASCII** → rustc ICE（`rustc-ice-*.txt`）。日志、注释只能放 `.rs`。
15. 杂项：文件名大小写（Windows 复制会出 `Trap.s` + 空 `trap.s`）；`linker.ld` 里 bss 符号用 `start_bss = .;`；riscv 锁 `rev=11d43cf`。

### D. 常量 / 语义细节
16. `CLOCK_FREQ` 必须与真实 mtime 频率一致（本机 QEMU = **10 MHz**）；`TICKS_PER_SEC = 100` → 时间片 `CLOCK_FREQ/TICKS_PER_SEC = 100000 ticks = 10ms`；填错会同时影响时间片长度和 `sys_get_time` 的微秒。
17. `MAX_APP_NUM=16` 比实际 app 数（4）大：数组会有 12 个“幽灵任务”，`TASK_MANAGER` 初始化时也会为它们建 `Ready`，但 `find_next_task` 用 `num_app` 取模，**不会被调度**（无害，可优化为 `for i in 0..num_app`）。
18. `run_next_task` 的 `else`（没有 Ready 任务）目前是 `panic!("All applications completed!")` —— 更贴合 ch3 语义的是 `println!` + `shutdown(false)` 优雅关机。
19. `task/mod.rs` 顶部 `use core::borrow::BorrowMut;` 已无用，可删。

## 5. rCore ch4 预习提纲（下次主题：**地址空间 / 分页**）

ch4 相对 ch3 的核心变化：**从“多个 app 共享同一片物理内存、各占一个固定槽位”升级为“每个 app 有独立的虚拟地址空间，靠 Sv39 页表隔离”**。预计会遇到：

1. **虚拟地址 / 物理地址 / Sv39 三级页表**：`satp` 寄存器、MMU 地址翻译、`sfence.vma` 刷 TLB；页表项 PTE 与权限位（R/W/X/U）。
2. **物理内存管理**：页帧分配器 `FrameAllocator`（4KB 页），`PAGE_SIZE`、`MEMORY_END` 等常量。
3. **逻辑段与映射类型**：`MapType`（`Identical` 恒等 / `Direct` / `Framed`）、`MapPermission`；内核地址空间 `KERNEL_SPACE`（含恒等映射段 + 直接映射段）。
4. **应用地址空间**：每个 app 一个 `MemorySet`（代码/数据/堆/栈 + 自定义段），`ELF` 解析加载（不再是简单 incbin 拷贝）。
5. **跳板 trampoline**：内核与用户**共享同一页**（`TRAMPOLINE`），`__alltraps`/`__restore` 移入跳板，`stvec` 指向它 —— 因为切地址空间后，取指必须落在“两边都映射到同一物理页”的代码上。
6. **TrapContext 迁移**：从“内核栈上”搬到**应用地址空间**里的 `TRAP_CONTEXT`（用 `trap_cx_ppn` 定位）。
7. **切换地址空间**：`TaskControlBlock` 增加 `satp` / `memory_set` / `kstack_base` 等；`__switch` 里同时切 `satp` + `sfence.vma`。
8. **内核访问用户数据**：不能直接解引用用户传进来的指针，需要 `translated_str` / `translated_byte_buffer` 做地址翻译 + 拷贝（否则 `sys_write` 又会读到乱七八糟的东西）。
9. **内核自己也用虚拟地址**：内核起始地址通常会移到**高地址**（如 `0xFFFFFFC0_80200000`），`linker.ld` 大改。
10. 预计新增/大改：`os/src/mm/{mod,memory_set,page_table,frame_allocator,address}.rs`；`trap/*` 改跳板版；`task/*` 改地址空间版；`config.rs` / `linker.ld` / `build.rs` 改；`user/linker.ld` 基址也变（地址空间独立后，所有 app 可链同一基址，`build.py` 逐 app 换基址可能不再需要）。

**建议动手前先巩固 ch3 这几点**：
- 两条控制流通道的分工：`__alltraps/__restore`（进出用户态，存全部寄存器） vs `__switch`（内核内换任务，只存 callee-saved）；
- `TaskContext{ra=__restore, sp=初始 TrapContext 地址}` 这个“首次进任务”的衔接点（ch4 会把这套搬进跳板，务必先理解透）；
- `__restore` 里为什么**不能**有 `mv sp, a0`；
- app 为什么必须在“它被加载的地址”上链接（ch4 用页表隔离后这条会以新形式出现）；
- `UPSafeCell` 的借用（`exclusive_access`）与“内核 SIE=0 不可被时钟抢占”的关系。

## 6. 新聊天开场白（可直接复制）
> "读取 `Odyssey/docs/CH4_交接文档.md` 和 repo memory 里 `odyssey-rcore-notes.md`，我们继续学 rCore ch4（地址空间与分页）。当前 os/user 在 WSL `/root/Odyssey`，ch3 抢占式分时多任务已完成并跑通。"
