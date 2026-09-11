mod context;
mod switch;

#[allow(clippy::module_inception)] // 显式允许该模块与父模块(task)同名
mod task;

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};



pub use crate::task::context::TaskContext;

// 任务管理器: 全局唯一实例 `TASK_MANAGER`，负责所有任务的状态迁移与切换。
pub struct TaskManager {
    // 实际内嵌的app数量（开机时从符号 `_num_app` 读出）
    num_app: usize,
    // 需要可变访问的状态，用 UPSafeCell 包一层；
    // 单核下通过 `exclusive_access()` 取得 &mut TaskManagerInner（借用是运行时检查）。
    inner: UPSafeCell<TaskManagerInner>,
}

// TaskManager 的内部可变状态： 任务表 + 当前任务下标
struct TaskManagerInner {
    // 定长数组，最多MAX_APP_NUM个TCB
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

// 运行时初始化全局变量
// 需要lazy_static!
lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();            // 取得内嵌的app数量
        let mut tasks = [TaskControlBlock {     // 初始化tasks
            task_cx: TaskContext::zero_init(),  // 占位：真正的上下文在下面循环里覆盖
            task_status: TaskStatus::UnInit,    // 初始状态：UnInit
        }; MAX_APP_NUM];
        // 为每个任务建立“出生现场”，并标记为可调度：
        //   init_app_cx(i)        —— 在第 i 块内核栈顶放好初始 TrapContext，并返回它的地址
        //   goto_restore(addr)    —— 把该地址包成 TaskContext(ra=__restore, sp=addr)，
        //                            使任务首次被 __switch 选中时能经 __restore 进入用户态
        //   status = Ready        —— 等待调度器挑选（首个任务稍后由 run_first_task 置为 Running）
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        // 赋值TaskManager
        TaskManager {
            num_app,
            // UPSafeCell::new 是 unsafe: 调用者需保证单核独占使用
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0, // 指向 0 号任务；此刻还没有任务在 Running，稍后 run_first_task 会把 0 号置为 Running
                })
            },
        }
    };
}

impl TaskManager {
    // 运行第一个任务
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        // 分配一块未使用的内存
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }
    // 暂停当前任务
    fn make_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }
    // 推出当前任务
    fn make_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }
    // 运行下一个任务
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(
                    current_task_cx_ptr,
                    next_task_cx_ptr,
                );
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }
    // 查找下一个任务
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| {
                inner.tasks[*id].task_status == TaskStatus::Ready
            })
    }
}

/* 
    对TASK_MANAGER的便捷入口
*/
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn make_current_suspended() {
    TASK_MANAGER.make_current_suspended();
}

fn make_current_exited() {
    TASK_MANAGER.make_current_exited();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

pub fn suspend_current_and_run_next() {
    make_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    make_current_exited();
    run_next_task();
}


