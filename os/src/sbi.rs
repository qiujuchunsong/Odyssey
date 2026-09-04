/* SBI函数 */
#[allow(deprecated)]                            // 抑制编译器对已弃用代码的警告。在项目中使用已弃用的功能或方法时，可以通过此属性避免警告。
pub fn console_getchar() -> Option<char> {      // 获取输入字符，无输入返回 None
    let ret = sbi_rt::legacy::console_getchar();
    if ret == usize::MAX {                      // -1 表示无输入
        None
    } else {
        Some((ret & 0xFF) as u8 as char)        // 取低8位转为字符
    }
}
#[allow(deprecated)]
pub fn console_putchar(c: usize) {              // 输出字符c
    sbi_rt::legacy::console_putchar(c);
}
pub fn shutdown(failure: bool) -> ! {           // 关机函数
    use sbi_rt::{system_reset, NoReason, Shutdown, SystemFailure};
    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }
    unreachable!()
}