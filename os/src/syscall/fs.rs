/* fs.rs(文件系统/输入输出类) 管文件*/

use crate::mm::translated_byte_buffer;
use crate::task::{current_user_token, suspend_current_and_run_next};
use crate::sbi::console_getchar;


const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;     // 标准输出模式

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_token(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            // console_getchar() 返回 Option<char>：None = 暂时没有输入。
            // 没输入时【必须让出 CPU】，不能死等 —— 否则这个进程会一直霸着
            // 时间片空转，shell 看起来像卡死（ch5 §6 预警里那条"忙等 + 让出"）。
            let c = loop {
                match console_getchar() {
                    Some(c) => break c,
                    None => suspend_current_and_run_next(),
                }
            };
            let ch = c as u8;
            let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
            unsafe { buffers[0].as_mut_ptr().write_volatile(ch); }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}
