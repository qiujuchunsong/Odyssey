
// 引入所需宏
use crate::config::*;
use crate::trap::TrapContext;
use core::arch::asm;

// 内核栈结构，每 4 * 1024 (4KB)一页
#[repr(align(4096))]
#[derive(Copy, Clone)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

// 实际分配栈所需内存
static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl KernelStack {
    // 获取内核栈栈顶指针
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    // 压栈操作
    pub fn push_context(&self, trap_cx: TrapContext) -> usize {
        let trap_cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *trap_cx_ptr = trap_cx;
        }
        trap_cx_ptr as usize
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

// 获取当前应用编号
fn get_base_i(app_id : usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

// 获取应用数量
pub fn get_num_app() -> usize {
    unsafe extern "C" {
        safe fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

// 加载应用
pub fn load_apps() {
    // 获取应用数量
    unsafe extern "C" {
        safe fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    // 建立app_start切片
    let app_start = unsafe {core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)};
    // 循环逐个加载
    for i in 0..num_app {
        let base_i = get_base_i(i);
        // 1) 清零整个槽位
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe{(addr as *mut u8).write_volatile(0) });
        // 2) 源 = 内核 .data 里的镜像
        let src = unsafe {
        core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        // 3) 目的 = 固定槽位
        let dst = unsafe {core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };
        // 4) 拷贝
        dst.copy_from_slice(src);
    }

    // 指令同步屏障
    // 刚把字节写进base_i(这段内存将来要被当指令执行)，必须 fence.i 保证之后的取指能看到之前的写入。
    unsafe {
        asm!("fence.i");
    }
}

// 为第 app_id 个任务构造初始化 TrapContext, 放在其内核栈栈顶下方并返回地址
pub fn init_app_cx(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        get_base_i(app_id),
        USER_STACK[app_id].get_sp(),
    ))
}