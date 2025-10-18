# 荣誉准则

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

# lab3实验中我实现的功能

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

# 简答作业

