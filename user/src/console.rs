// 用户态程序不能直接调用 SBI(那是内核/M态的特权),必须通过系统调用让内核代为输出
use super::write;                           // 引入 user_lib 顶层的 write 函数(内部封装 sys_write)
/* fmt模块文档[https://doc.rust-lang.org/stable/std/fmt/index.html] */
use core::fmt::{self, Write};               // 引入core库中的fmt模块和Write trait

struct Stdout;                              // 创建结构体Stdout,作为标准输出的实现

const STDOUT: usize = 1;                              // 定义常量STDOUT,表示标准输出的文件描述符

impl Write for Stdout {                     // 赋予Stdout Write trait
    fn write_str(&mut self, s: &str) -> fmt::Result {   
        // 创建write_str函数,输出字符串s中的每个字符
        // fmt::Result是一个类型别名，表示格式化操作的结果类型。它是一个Result类型，表示操作可能成功或失败。
        write(STDOUT, s.as_bytes());  // 调用write函数,将字符串s转换为字节数组并写入标准输出
        Ok(())                                         // 返回Ok(())
    }
}

pub fn print(args: fmt::Arguments) {        // public函数print,接受fmt::Arguments类型的参数args
    Stdout.write_fmt(args).unwrap();        // 使用Stdout的write_fmt输出args,如果输出失败则调用unwrap()直接panic崩溃
}

#[macro_export]                             // 导出宏
macro_rules! print {                        // 定义print宏,接受一个格式字符串和可选的参数
    ($fmt: literal $(, $($arg: tt)+)?) => { // $fmt: literal表示宏的第一个参数是一个字面量格式字符串，
                                            // $(, $($arg: tt)+)?表示宏可以接受零个或多个额外的参数，这些参数可以是任意的Rust语法树片段（token tree）。
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));    // $crate表示当前crate的根模块，console::print表示调用console模块中的print函数，format_args!宏用于格式化输出
                                                                      // $fmt $(, $($arg)+)?表示将宏的参数原样传递给format_args!宏进行格式化
    }// 格式化输出
}

#[macro_export]                             // 导出宏
macro_rules! println {                      // 定义println宏,接受一个格式字符串和可选的参数
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));   // concat!($fmt, "\n")表示将格式字符串与换行符拼接，形成新的格式字符串
    }
}
