# 实验三

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 助教Dynamic_Pigeon交流：
   >
   > 我：中途遇到一个抽象的问题，实在是匪夷所思。还是关于这个crate和super的，之前我对这两个的理解是一个表示绝对路径，一个表示相对路径。这次在引入其他模块的函数（其父目录相同）的时候，我在 mod.rs 中引入这个函数用到super我能理解。在非mod.rs中引入这个函数居然要用crate
   > 这里用super引用这个函数（get_current_syscall_stats），居然还报错了
   >
   > 助教Dynamic_Pigeon：非mod.rs的super是这个文件夹的mod.rs。
   >
   > Jiang Sheng交流：
   >
   > 代码调试相关操作，以及对部分内核结构的讨论。

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > ### Rust 官方标准库文档（`BTreeMap` 具体说明）
   >
   > - **网址**：https://doc.rust-lang.org/stable/core/collections/btree_map/struct.BTreeMap.html
   > - **内容**：详细介绍 `BTreeMap` 的结构体定义、方法（如 `insert`、`get`、`entry` 等）、特性（基于 B 树实现，有序存储、支持范围查询）以及使用示例。

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

------

## lab1实验中我实现的功能

我所实现的功能如下：

1. 在`./os/src/task/task.rs`文件夹下的`TaskControlBlock`中加入了基于`BTreeMap<usize, usize>`数据结构的系统调用统计。

2. 在`./os/src/task/task.rs`文件中，由于`BTreeMap<usize, usize>`不兼容`#[derive(copy)]`因此将原来的`#[derive(copy, Clone)]`改为`#[derive(Clone)]`

3. 由于2的修改，因此修改了`./os/src/task/mod.rs`文件夹下`lazy_static!`中`tasks`初始化的操作。取代原有通过copy的方式给数组初始化，而改为逐一赋值的方式。

4. 为`./os/src/task/mod.rs`文件夹下的`TaskManager`结构体添加了两个方法——记录当前任务的系统调用、返回当前任务的系统调用统计。

5. 通过2中的两个方法成功完善了

   > `fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize`
   >
   > - 如果 `trace_request` 为 4，表示查询当前任务调用编号为 `id` 的系统调用的次数，返回值为这个调用次数。**本次调用也计入统计** 。

4. 完善了获取任务信息的其他调用规范。

------

## 简答作业

### 问题一解答：

#### 报错日志：

`[kernel] Loading app_0`
`[kernel] PageFault in application, kernel killed it.`
`[kernel] Loading app_1`
`[kernel] IllegalInstruction in application, kernel killed it.`
`[kernel] Loading app_2`
`[kernel] IllegalInstruction in application, kernel killed it.`

#### 使用的 sbi 及其版本：

`[rustsbi] RustSBI version 0.2.2, adapting to RISC-V SBI v1.0.0`

`[rustsbi] Implementation: RustSBI-QEMU Version 0.1.1`

当前系统运行的是适配 `RISC-V SBI v1.0.0` 规范的 `RustSBI` 实现，其中 `RustSBI` 核心版本 `0.2.2`，具体用于 `QEMU` 的变体版本为 `0.1.1`。

#### 描述程序出错行为：


##### 1. `app_0`：触发“页错误（`PageFault`）”被内核终止
出现`“PageFault in application”`，说明程序在访问内存地址时，违反了内存管理规则，导致CPU抛出“页错误”异常。内核捕获到这个异常后，判定为应用程序非法操作，最后终止了这个进程。


##### 2. `app_1` 与 `app_2`：触发“非法指令（`IllegalInstruction`）”被内核终止
`“Loading app_1”“Loading app_2”`两者都出现`“IllegalInstruction in application”`，表示程序在执行CPU指令时，指令本身不符合当前CPU的指令集规则，CPU抛出“非法指令”异常。内核捕获异常后，判定程序无法正常执行，最后终止了这个进程。

### 问题二解答：

#### 1. L40：刚进入 `__restore` 时，`sp` 代表了什么值。请指出 `__restore` 的两种使用情景。

刚进入 `__restore` 时，`sp` 指向用户态上下文栈帧的起始地址

 `__restore`的两种使用情景：

1. 内核处理完用户态触发的系统调用（`ecall`）后，恢复用户态执行；
2. 内核处理完用户态的异常或中断后，返回用户态继续运行。

#### 2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

代码特殊处理了三个控制状态寄存器（`CSR`）：`sstatus`、`sepc`、`sscratch`，以及从栈帧中加载的对应值。

`sstatus`：通过 `csrw sstatus, t0` 恢复。`sstatus` 控制特权级状态（如`SPP`位记录 trap 前的特权级，此处需设为 U 态）和中断使能（`SIE`位）。恢复后，确保返回时处理器识别 “从 S 态回到 U 态”。

`sepc`：通过 `csrw sepc, t1` 恢复。`sepc` 保存 trap 发生时用户态的指令地址，恢复后，`sret` 指令会跳转到该地址，保证用户程序从断点继续执行。

`sscratch`：通过 `csrw sscratch, t2` 恢复。`sscratch` 通常保存用户态的栈指针（`x2`），恢复后，下次发生 trap 时，内核可通过`sscratch`快速切换到内核栈。

#### 3. L50-L56：为何跳过了 `x2` 和 `x4`？

`x2` 是栈指针（`sp`）：用户态的`sp`需在最后通过与`sscratch`交换恢复（避免过早覆盖内核栈指针导致上下文丢失），因此此处跳过。

`x4` 是线程指针（`tp`）：`tp` 由用户态程序自行维护（用于线程局部存储），内核不参与修改，且 trap 过程中`tp`的值未被破坏，因此无需从栈帧恢复。

#### 4. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

`csrrw`指令的作用是交换`sp`和`sscratch`的值。交换后`sp` 变为**用户态的栈指针**（从`sscratch`中取出，`sscratch`在此前已被恢复为用户态`x2`的值），确保用户态执行时使用自己的栈。`sscratch` 变为**内核态的栈指针**（即进入`__restore`时的`sp`，指向用户态上下文栈帧），用于下次 trap 发生时快速切换到内核栈。

#### 5.  `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

状态切换发生在 `sret` 指令（通常在`__restore`末尾）。

因为`sret` 是从 S 态 trap 处理返回的专用指令，其行为由`sstatus`寄存器的`SPP`位决定。若`SPP`位为 0（表示 trap 前是 U 态），`sret` 会将特权级切换回 U 态，并跳转到`sepc`保存的用户态指令地址，因此执行后进入用户态。

#### 6. L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

交换后`sp` 变为**内核态的栈指针**（从`sscratch`中取出，`sscratch`预先保存了内核栈地址），用于后续保存用户态寄存器到 trap frame。`sscratch` 变为**用户态的栈指针**（即 trap 发生时的`x2`），后续会被保存到 trap frame 中，供`__restore`恢复。

#### 7. 从 U 态进入 S 态是哪一条指令发生的？

`ecall`（用户态系统调用指令）
