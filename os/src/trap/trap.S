.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm

.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm

    .section .text
    .globl __alltraps
    .globl __restore
    .align 2
    
__alltraps:
    csrrw sp, sscratch, sp // 保存 sscratch 的旧值，将 sp 的值写入 sscratch， 把保存的旧值写入 sp
    # now sp->kernel stack, sscratch->user stack
    # allocate a TrapContext on kernel stack
    addi sp, sp, -34*8

    // 把通用寄存器保存到栈上
    # save general-purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr

    // 把 csr 保存到通用寄存器，再保存到栈上
    # we can use t0/t1/t2 freely, because they were saved on kernel stack
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)


    # read user stack from sscratch and save it on the kernel stack
    csrr t2, sscratch // sscratch 现在指向用户栈
    sd t2, 2*8(sp)

    # set input argument of trap_handler(cx: &mut TrapContext)
    mv a0, sp
    call trap_handler // 跳转到 trap_handler

__restore:
    # case1: start running app by __restore
    # case2: back to U after handling trap
    mv sp, a0 // a0 指向 TrapContext
    # now sp->kernel stack(after allocated), sscratch->user stack

    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2

    # restore general-purpuse registers except sp/tp
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
    sret // 开始执行用户程序
