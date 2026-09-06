
mod context;    // 引入context,提供TrapContext（陷入上下文）类型

use crate::batch::run_next_app;     // 使用batch中的run_next_app
use crate::syscall::syscall;
use core::arch::global_asm;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval, stvec,
};

global_asm!(include_str!("trap.s"));    // 内联 trap.s 汇编: __alltraps 负责保存现场, __restore 负责恢复。
                                         // 它服务所有 trap(异常/中断); __restore 还被 batch 用来启动 app

pub fn init() {
    unsafe extern "C" {
        safe fn __alltraps();   // 获取__alltraps()的地址
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);    // 把 stvec 设为 __alltraps 地址
                                                                // Direct 模式: 所有 Trap 一律跳到该入口(stvec 低2位=00)
                                                                // 注意: 不是 Vectored 那种 BASE + 4*cause 向量表
    }
}

// 陷入处理器函数，根据不同的Trap原因采取不同的处理方式
#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    // 读取scause(Trap原因)和stval(Trap附加信息)
    let scause = scause::read();
    let stval = stval::read();
    // 根据Trap原因采取不同措施
    match scause.cause() {
        // 如果是用户态系统调用，则令sepc(当 Trap 是一个异常的时候，记录 Trap 发生之前执行的最后一条指令的地址)地址加4
        // 使得在返回用户态时可执行下一条指令
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            // 执行本次 ecall 对应的系统调用, 返回值写回用户 a0(x10)
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        // 如果是存储错误，则提示发生存储错误并执行下一个app
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            run_next_app();
        }
        // 如果是非法指令调用，则提示发生非法指令调用并执行下一个app
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            run_next_app();
        }
        // 如果都不满足，则显示未支持的Trap，并打印scause(原因)和stval(附加信息)
        _=> {
            panic!(
                "Unsupported trap {:#?}, stval = {:#x}!",
                scause.cause(),
                stval);
        }
    }
    cx  // 返回TrapContext

}

pub use context::TrapContext;    // 对外再导出 TrapContext, 供 batch 等模块使用