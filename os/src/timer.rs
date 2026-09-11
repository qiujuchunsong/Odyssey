use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;       // 每秒时间片数  100 -> 10ms/片
const MICRO_PER_SEC: usize = 1_000_000; // 一秒由多少微秒

/// 获取当前计数器值(ticks)
pub fn get_time() -> usize {
    time::read()
}

/// 以微秒为单位获取当前时间值
pub fn get_time_us() -> usize {
    time::read() / (CLOCK_FREQ / MICRO_PER_SEC)
}

/// 设置下一次中断，当前 mtime 往后CLOCK_FREQ / TICKS_PER_SEC 个 tick
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC); // CLOCK_FREQ: tick <-> 秒
                                                        // 每秒时间片数
}
