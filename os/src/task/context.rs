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
    pub fn goto_restore(kstack_ptr: usize) -> Self {
        unsafe extern "C" {     // 取得__restore的地址
            unsafe fn __restore();
        }
        Self {
            ra: __restore as usize, // 新任务首次被调度后，第一步就是“回到用户态”
            sp: kstack_ptr,         // 初始 TrapContext 在内核栈上的地址（由 `init_app_cx` 放置）。
            s: [0; 12],             // 新任务无需恢复callee-saved寄存器
        }
    }
}