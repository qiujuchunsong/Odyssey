use core::arch::asm;        // 使用asm宏，使该模块能够使用汇编语言

/* 定义系统调用编号 */
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

/* 封装syscall()系统调用函数
   其本质为：把"我要干什么"写进寄存器，执行ecall。CPU硬件立刻陷入S态
*/
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;     // ret是接住系统调用返回值的变量
    unsafe {
        asm!(
            "ecall",        // 执行ecall命令
            /*  x10~x12(a0~a2)表示系统调用参数
                x17(a7)表示系统调用ID
             */
            inlateout("x10") args[0] => ret,  // in + lateout 先供汇编指令读，等汇编代码读取完所有的in寄存器后，该寄存器被写入新的值，作为输出
            in("x11") args[1],                // in 表示将Rust变量的值传入汇编代码中
            in("x12") args[2],
            in("x17") id
        );
    }
    ret

/* fd表示文件描述符
buffer.as_ptr() as usize获取字节切片首地址，buffer.len*()为字节数。因为内核态无法直接借用用户态的引用，所以要拆成起始地址与长度两个数字，后续在内核中会被拼接 */
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}
/* exit_code是退出码，0表示成功。0, 0是占位，exit系统调用只需要一个参数，但syscall()固定收3个，后两个内核根本不会看
 */
pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}