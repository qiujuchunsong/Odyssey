// 引入所用的宏
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use core::arch::asm;
use lazy_static::*;             // lazy_static宏提供了全局变量的运行时初始化功能，使得App_Manager可以在运行时再被初始化

const USER_STACK_SIZE : usize = 4096 * 2;    // 分配用户栈（8KB）
const KERNEL_STACK_SIZE : usize = 4096 * 2;  // 分配内核栈（8KB）
const MAX_APP_NUM : usize = 16;              // 最大应用数
const APP_BASE_ADDRESS : usize = 0x80400000; // 应用初始地址
const APP_SIZE_LIMIT : usize = 0x20000;      // 最大应用大小

#[repr(align(4096))]    // 要求这块内存的起始地址按4KB（一页）对齐，方便映射，规范布局
struct KernelStack {    // 本质没有“栈”的逻辑，只是包着一块8KB个u8的连续内存块。下同
    data : [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data : [u8; USER_STACK_SIZE],
}

static KERNEL_STACK : KernelStack = KernelStack {   // static 表示这两块内存在编译期就分配好，地址固定
    data : [0; KERNEL_STACK_SIZE],                  // 换而言之，内核镜像里永久地划出两个8KB，一块专供内核，一块专供应用
};

static USER_STACK : UserStack = UserStack {
    data : [0; USER_STACK_SIZE],
};

/* KernelStack 的方法：取栈顶 + 把初始 TrapContext 写到栈顶 */
impl KernelStack {
    // 返回本栈块最高地址，作为初始 sp（RISC-V 栈向低地址增长，sp 从最高地址开始）
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    // 作用：把一个"CPU 现场"(TrapContext) 写到内核栈顶正下方，
    //       供 __restore 从该地址恢复，从而以该现场进入/回到用户态
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        // ① 放置地址 = 栈顶 - TrapContext 大小（32个寄存器+sstatus+sepc = 34*8 = 272B）
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;                          // ② 把 cx 整体写入这块内存（按值 move）
        }
        unsafe { cx_ptr.as_mut().unwrap() }        // ③ 裸指针 → &mut 引用，交给调用方
    }
}

/* UserStack 的方法：返回用户栈顶（即 app 的初始 sp） */
impl UserStack {
    // 返回本栈块最高地址，作为 app 初始 sp（RISC-V 栈向低地址增长）
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

/* 应用管理器：记录应用总数、当前运行到第几个，以及每个应用的字节区间。
   区间表示法: 第 i 个应用的字节范围是 [app_start[i], app_start[i+1])。
   所以 app_start 长度要 MAX_APP_NUM + 1 —— 末尾多存一个"结束地址"作哨兵 */
struct  AppManager {
    num_app : usize,          // 应用总数（内嵌了几个 app）
    current_app : usize,      // 下一个将要运行的应用编号（初始 0）
    app_start : [usize; MAX_APP_NUM + 1],   // 每个应用的起止地址（末尾是哨兵）
}
/* AppManager 的方法 */
impl AppManager {
    // 打印应用信息
    pub fn print_app_info(&self) {
        println!("[kernel] num_app : {}", self.num_app);
        for i in 0..self.num_app {
            println!("[kernel] app_{} [{:#x}, {:#x})",
                    i,
                    self.app_start[i],
                    self.app_start[i + 1]
                );
        }
    }
    // 加载应用
    fn load_app(&self, app_id : usize) {
        if app_id >= self.num_app { // 如果应用id超出当前应用数量，显示所有应用已完成，关机
            println!("All applications completed!");
            shutdown(false);
        }
        println!("[kernel] loading app_{}", app_id);    // 显示加载应用
        unsafe {
            core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0); // ① 清零整个加载区：防止上一个更长的 app 残留"尾部字节"变成幽灵代码
            let app_src = core::slice::from_raw_parts( // 获取应用镜像
                self.app_start[app_id] as *const u8,
                self.app_start[app_id + 1] - self.app_start[app_id]
            );
            let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len()); // 得到应用加载位置
            app_dst.copy_from_slice(app_src); // 加载应用
            asm!("fence.i"); // 保证取指过程能够看到之前对于取指内存区域的修改
        }
    }

    // 获取当前应用编号
    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    // 把"下一个待运行编号"后移一位：表示当前这个 app 已被交付运行
    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }

}

/* 全局唯一的应用管理器 APP_MANAGER。
   为什么要 lazy_static!: AppManager 要在运行时才能初始化——它得先读出
   "_num_app 符号所在内存里的值"(该值由链接期布局 + build.rs 决定, 编译期不知道),
   而 Rust 的 static 要求"编译期就能算出的常量", 所以必须延迟到第一次被访问时才初始化。
   外层再包 UPSafeCell: 提供单核下安全可变访问(exclusive_access)。 */
lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            // 把链接脚本符号 _num_app"伪装成函数取地址", 从而得到它在内存中的位置
            unsafe extern "C" {
                safe fn _num_app();      // 只取地址, 从不真正调用
            }
            // build.rs 生成的内存布局: [_num_app: 应用总数][app_0_start][app_0_end][app_1_start]...
            //                                   ↑ num_app_ptr 指向这里(一个 usize = 总数)
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();   // 读出应用总数
            // 紧随其后的 num_app+1 个 usize 是各应用的起/止边界(末尾含哨兵)
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            let app_start_raw: &[usize] =
                core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);
            app_start[..=num_app].copy_from_slice(app_start_raw);   // 拷进定长数组备用
            AppManager {
                num_app,
                current_app: 0,     // 从第 0 个应用开始
                app_start,
            }
        })
    };
}

/// init batch subsystem
pub fn init() {
    print_app_info();
}

/// print apps info
pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

/// run next app
pub fn run_next_app() -> ! {    // 永不返回! 一旦调用run_next_app()则直接跑下一个app。对用 "批处理"的含义:应用出错了直接杀，换下一个
    let mut app_manager = APP_MANAGER.exclusive_access();
    let current_app = app_manager.get_current_app();
    app_manager.load_app(current_app);
    app_manager.move_to_next_app();
    drop(app_manager);
    // before this we have to drop local variables related to resources manually
    // and release the resources
    unsafe extern "C" {
        unsafe fn __restore(cx_addr: usize);
    }
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }
    panic!("Unreachable in batch::run_current_app!");
}