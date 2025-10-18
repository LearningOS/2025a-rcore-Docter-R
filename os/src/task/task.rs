//! Types related to task management

use super::TaskContext;

// 本人添加
use alloc::collections::BTreeMap;

/// The task control block (TCB) of a task.
#[derive(Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// 本人添加：系统调用统计（键：系统调用编号，值：调用次数）
    pub syscall_stats: BTreeMap<usize, usize>,

}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
