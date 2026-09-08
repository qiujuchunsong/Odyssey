# Odyssey · rCore 学习交接文档（ch2 完成 → ch3 待学）

> 用途：在新聊天中继续学 rCore ch3 时，先让 Copilot 读取本文件 + `/memories/repo/odyssey-rcore-notes.md`，即可无缝衔接。
> 生成日期：2026-09-06

## 1. 当前状态一句话
rCore **ch2 批处理系统（Batch OS）已学完**：`os` 内核 + `user` 用户库双 crate 可编译通过；`user` 有 5 个 app（00hello_world ~ 04priv_csr）内嵌进内核；内核能加载并依次运行 app、处理 `ecall` 系统调用（write/exit）、在 app 出错（page fault / 非法指令 / 特权指令）时打印 `kernel killed it` 并切换下一个。

## 2. 环境与命令（重要）
- 工具链：`Odyssey/rust-toolchain.toml` 锁 **`nightly-2025-02-18`**（stable 编不过 `riscv 0.6.0` 的 `asm_const`）
- 依赖版本：`os` 与 `user` 都用 `riscv = { git = ..., rev = "11d43cf...", features = ["inline-asm"] }`（0.6.0；**勿用** crates.io 新版 riscv 0.16，API 完全不同）
- 编译顺序：`cd user && make build`（生成全部 .bin）→ `cd os && cargo build`
- 运行：`cd os && make run`（qemu-system-riscv64）
- 从 VS Code PowerShell 调 wsl 的坑：PATH 别拼接、`$` 会被外层展开、别经 PowerShell 跑复杂 perl（会把换行删光）→ 用完整路径 `/usr/bin/make OBJCOPY='/root/.cargo/bin/rust-objcopy --binary-architecture=riscv64' build`

## 3. 代码地图
```
Odyssey/
├─ rust-toolchain.toml        # 锁 nightly-2025-02-18
├─ os/                        # 内核(S态)
│  ├─ build.rs                # 生成 src/link_app.S（把 user 的 .bin 内嵌进内核）
│  ├─ src/main.rs             # rust_main: clear_bss → trap::init → batch::init → batch::run_next_app
│  ├─ src/entry.asm           # _start: 设 boot_stack → call rust_main（纯 ASCII）
│  ├─ src/linker.ld           # 内核 0x80200000 布局 + 段符号(stext/..sbss/ebss/boot_stack)
│  ├─ src/console.rs          # println!/print! → sbi::console_putchar
│  ├─ src/sbi.rs              # console_putchar/shutdown(经 sbi_rt 到 M 态)
│  ├─ src/lang_items.rs       # panic_handler → shutdown
│  ├─ src/sync/{mod,up}.rs    # UPSafeCell(RefCell+unsafe impl Sync, 单核可变 static)
│  ├─ src/batch.rs            # AppManager / KernelStack / UserStack / load_app / run_next_app
│  ├─ src/trap/{mod,context.rs,trap.s}
│  │                          # stvec=__alltraps; trap_handler; TrapContext(34槽);
│  │                          # trap.s: __alltraps(存现场)/__restore(恢复或启动app)（纯 ASCII）
│  └─ src/syscall/{mod,fs,process}.rs  # syscall() 分发; sys_write(fs)/sys_exit(process)
└─ user/                       # 应用库(U态, no_std)
   ├─ src/lib.rs              # _start: clear_bss→exit(main()); weak main; write/exit
   ├─ src/syscall.rs          # asm ecall: x10~12 参数, x17=id, x10 返回
   ├─ src/console.rs          # println! → write(1, bytes)
   ├─ src/linker.ld           # app 基址 0x80400000
   └─ src/bin/*.rs            # 00hello_world ~ 04priv_csr（后4个是测试用）
```

## 4. 已踩过的坑（新会话别重踩）
1. **asm 文件（.s/.S）内不能有中文/非 ASCII** → rustc ICE（rustc-ice-*.txt，mbc 断言崩溃）。中文注释放 `.rs` 里没问题。
2. riscv 依赖要锁 `rev=11d43cf`（0.6.0），否则 API 不匹配（scause::Exception / stvec::write 签名等全变）。
3. `linker.ld` 里定义 bss 符号用 `start_bss = .;`（位置计数器赋符号），不是 `. = start_bss;`。
4. 文件名大小写：Windows 复制文件可能产生 `Trap.s` 与空 `trap.s` 并存 → include 到空文件 → undefined symbol。
5. `user_lib` 任何编译错误都会导致某些 app 的 .bin 缺失 → os 报 `Could not find incbin file`。
6. `rust-objcopy` 在 `/root/.cargo/bin`，make 子 shell 可能找不到 → 用完整路径覆盖 `OBJCOPY`。

## 5. rCore ch3 预习提纲（下次学习主题：多道程序 + 时间片/抢占式调度）
ch3 相对 ch2 的核心变化：**从"一个跑完再下一个"变成"多个应用按时间片轮流跑"**。预计会遇到这些新概念/文件：

1. **任务控制块 TCB / `TaskManager`**：每个"正在运行/等待运行的应用"变成一个任务对象（不再是简单计数器）。
2. **任务上下文切换 `__switch`**：一套**不走 trap** 的切换汇编（只保存/恢复被调用者保存寄存器 callee-saved: s0~s11 + ra + sp），用于内核在两个任务栈之间切换。→ 会新增类似 `os/src/task/switch.S`、`switch.rs`。
3. **`sys_yield` 系统调用**：应用主动让出 CPU（配合 `run_next_app` 演化为调度器）。
4. **时钟中断（抢占式）**：`mtime`/`mtimecmp`（经 SBI `set_timer`），开启 `sstatus.SIE`/`sie`，trap_handler 里区分"来自 U 的外部中断 Timer"。`sys_get_time` 获取时间。
5. **调度器**：FIFO / 轮转 Round-Robin，`TaskManager` 用循环队列或链表。
6. `batch.rs` 会大幅重构/被 `task.rs` 取代；`TrapContext` 与 `__switch` 分工要分清（一个是进出用户态，一个是内核内切换任务）。

**建议动手前先巩固 ch2 这几点**：`run_next_app` 的 `drop` 时序、`TrapContext` 34 槽布局、`__restore` 两条入口路径、UPSafeCell 为什么够用（单核无抢占——ch3 加时钟中断后要考虑临界区）。

## 6. 新聊天开场白（可直接复制）
> "读取 `Odyssey/docs/CH3_交接文档.md` 和 repo memory 里 `odyssey-rcore-notes.md`，我们继续学 rCore ch3（多道程序与抢占式调度）。当前 os/user 在 WSL `/root/Odyssey`，ch2 批处理已完成。"
