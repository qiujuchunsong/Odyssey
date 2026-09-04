# ============================================================
#  Odyssey OS — Makefile
#  用法:  make run   -> 编译并在 QEMU 中运行
#         make debug -> 运行并开启 GDB 调试端口
#         make build -> 只编译
#         make clean -> 清理产物
#  注意: 规则里的命令必须以【Tab】开头，不能用空格！
# ============================================================

# ---------------- 可配置变量 ----------------
# 注意: 注释不要写在赋值行末尾! 行尾空格会被保留进变量值
# Rust 编译目标（裸机 RISC-V）
TARGET := riscv64gc-unknown-none-elf
# 构建模式: debug / release
MODE := debug
# 内核 ELF 产物路径
KERNEL := target/$(TARGET)/$(MODE)/os
# QEMU 模拟器命令
QEMU := qemu-system-riscv64
# QEMU 机器类型
MACHINE := virt

# 根据 MODE 决定是否加 --release
ifeq ($(MODE), release)
CARGO_FLAGS := --release
else
CARGO_FLAGS :=
endif

# ---------------- 伪目标 ----------------
.PHONY: build run debug clean

# 只编译内核
build:
	cargo build $(CARGO_FLAGS)

# 编译并运行（run 依赖 build，会先自动编译）
run: build
	$(QEMU) -machine $(MACHINE) -nographic -bios default -kernel $(KERNEL)

# 运行并暂停等待 GDB 连接（-s -S: 开 1234 端口 / 启动即暂停）
debug: build
	$(QEMU) -machine $(MACHINE) -nographic -bios default -kernel $(KERNEL) -s -S

# 清理所有编译产物
clean:
	cargo clean
