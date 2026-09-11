// 声明并包含任务切换汇编 switch.S(__switch)

use crate::task::context::TaskContext;  // __switch的参数类型
use core::arch::global_asm;             // 引入global_asm! 宏

// 编译器把switch.S 嵌进本crate
global_asm!(include_str!("switch.S"));

unsafe extern "C" {
    pub unsafe fn __switch(
        // __switch(当前任务上下, 下一任务上下文)
        // 只保存/恢复ra, sp, s0~s11(callee-saved)--因为是一次普通函数调用
        current_task_cx_ptr: *mut TaskContext,
        next_task_cx_ptr: *const TaskContext,
    );
}