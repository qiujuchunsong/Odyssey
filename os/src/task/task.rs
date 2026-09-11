use crate::task::context::TaskContext;

// 自动派生Copy/Clone/PartialEq：便于按值传递、用 == 比较
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    // UnInit -> Ready -> Running -> Exited
    UnInit,  // 未初始化
    Ready,   // 就绪
    Running, // 运行
    Exited,  // 退出
}

// 任务控制块 包含任务状态与任务上下文
// 派生 Copy 是必需的：task/mod.rs 用 [TaskControlBlock{..}; MAX_APP_NUM] 初始化数组需要它
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,    // 任务状态
    pub task_cx: TaskContext,       // 任务上下文（内核栈 + 返回地址 + callee-saved）
}