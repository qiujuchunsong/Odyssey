
pub const USER_STACK_SIZE: usize = 4096 * 2;        // 用户栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;      // 内核栈大小
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const MAX_APP_NUM: usize = 16;                  // 最大应用数量
pub const APP_BASE_ADDRESS: usize = 0x80400000;     // 应用基地址
pub const APP_SIZE_LIMIT: usize = 0x20000;          // 应用最大大小

// time slice = CLOCK_FREQ / TICKS_PER_SEC ticks = 1 / TICKS_PER_SEC s
pub const CLOCK_FREQ: usize = 10_000_000;           // QEMU virt 的 mtime 频率：每秒 10_000_000 个 tick（1 tick = 0.1 µs）

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SIZE_BITS: usize = 12;
pub const MEMORY_END: usize = 0x80800000;
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;
pub const MMIO: &[(usize, usize)] = &[(0x0010_0000, 0x002000)];

pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}