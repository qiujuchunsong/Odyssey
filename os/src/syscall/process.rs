/* process.rs(进程/任务管理类) 管任务 */


use crate::batch::run_next_app;

pub fn sys_exit(xstate: i32) -> ! {
    println!("[kernel] Application exited with code {}", xstate);
    run_next_app()
}