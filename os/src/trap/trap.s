.altmacro   // 开启"高级宏模式"，允许使用 \n*8(算数运算) 和 %n(占位符传参)
.macro SAVE_GP n
    sd x\n, \n*8(sp) // 把寄存器 x{n} 存到 栈偏移 n*8 处
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp) // 从栈偏移 n*8 处 恢复寄存器 x{n}
.endm
    .section .text      // 将下面内容放进.text段(可读，可运行)
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    csrrw sp, sscratch, sp  // csrrw rd, csr, rs 的动作：rd = 旧csr; csr = rs。所以这行 = sp 变成 sscratch（内核栈顶），sscratch 变成旧 sp（用户栈）
    # now sp->kernel stack, sscratch->user stack
    # allocate a TrapContext on kernel stack
    addi sp, sp, -34*8  // 内核栈往下挪272B，挖出34个槽保存上下文
    # save general-purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5       // 从x5开始
    .rept 27        // 重复27次
        SAVE_GP %n  
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they were saved on kernel stack
    csrr t0, sstatus    // 读sstatus(发生Trap,CPU处于哪个特权级)到临时寄存器t0(已保存在内核栈)
    csrr t1, sepc       // 读sepc(被打断的指令地址)到临时寄存器t1(已保存在内核栈)
    sd t0, 32*8(sp)     // 存到槽32
    sd t1, 33*8(sp)     // 存到槽33
    # read user stack from sscratch and save it on the kernel stack
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # set input argument of trap_handler(cx: &mut TrapContext)
    mv a0, sp           // a0 = x10 是系统调用的第一个参数
    call trap_handler

__restore:
    # case1: start running app by __restore
    # case2: back to U after handling trap
    mv sp, a0
    # now sp->kernel stack(after allocated), sscratch->user stack
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    # restore general-purpuse registers except sp(x2)/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # release TrapContext on kernel stack
    addi sp, sp, 34*8
    # now sp->kernel stack, sscratch->user stack
    csrrw sp, sscratch, sp
    sret
