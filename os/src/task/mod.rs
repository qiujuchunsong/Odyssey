mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;




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
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

// 运行时初始化全局变量
// 需要lazy_static!
lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(
                get_app_data(i),
                i,
            ));
        }
       TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
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
            // 【ch4 变化】没有 Ready 任务 = 所有应用都跑完了 → 优雅关机
            //   ch3 这里是 panic!("All applications completed!")：
            //     panic 虽然也会走到 shutdown，但会打出难看的 panic 信息、
            //     而且语义上"任务跑完"根本不是错误。
            //   所以改成：打印一句 + shutdown(false)（false = 正常关机，不是故障）
            println!("All applications completed!");
            shutdown(false);
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

    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
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

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

