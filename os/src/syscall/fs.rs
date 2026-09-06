/* fs.rs(文件系统/输入输出类) 管文件*/

const FD_STDOUT: usize = 1;     // 标准输出模式

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    // 匹配fd
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };   // 拼接用户态传来的起始地址和文本长度
            let str = core::str::from_utf8(slice).unwrap();                 // 读取拼接后的文本
            print!("{}", str);                                              // 打印
            len as isize
        },
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}