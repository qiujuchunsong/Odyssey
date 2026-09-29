use crate::trap::trap_return;

// TaskContext 任务上下文,保存ra, sp, s0 ~ s11 由被调用者保存的寄存器
#[derive(Copy, Clone)]  // 赋予TaskContext默认的复制和克隆特性
#[repr(C)]              // 以C语言的对齐规则插入填充,使ra/sp/s0~s11的内存布局严格对应switch.S里的偏移
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    /// init task context
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// set task context {__restore ASM funciton, kernel stack, s_0..12 }
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}