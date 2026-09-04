use core::panic::PanicInfo; // 引入core库中的panic模块和PanicInfo结构体，用于处理程序在运行时发生的恐慌（panic）情况。PanicInfo结构体包含了关于恐慌的详细信息，例如恐慌发生的位置和相关的错误消息。

#[panic_handler]            // 定义一个自定义的恐慌处理函数，当程序发生恐慌时会调用该函数
fn panic(info: &PanicInfo ) -> ! {
    if let Some(location) = info.location() {
        crate::println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    }
    else {
        crate::println!("Panicked : {}", info.message());
    }
    // 用户态没有 shutdown 权限, panic 后只能挂起(死循环);
    // 等 os 实现 sys_exit 后可改为 crate::exit(-1) 让内核回收当前应用
    loop {}
}