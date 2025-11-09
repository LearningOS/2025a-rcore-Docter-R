//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use crate::task::current_task;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    /// 在当前任务的页表中映射一段虚拟地址区域    
    pub fn sys_mmap_tcb(&mut self, start: usize, len: usize, prot: usize) -> isize {
        if let Some(task) = current_task() {
            let mut inner = task.inner_exclusive_access();
            inner.sys_mmap_tcb(start, len, prot)
        } else {
            -1
        }
    }
    /// 在当前任务的页表中取消映射一段虚拟地址区域
    pub fn sys_munmap_tcb(&mut self, start: usize, len: usize) -> isize {
        if let Some(task) = current_task() {
            let mut inner = task.inner_exclusive_access();
            inner.sys_munmap_tcb(start, len)
        } else {
            -1
        }
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

/// 在当前任务的页表中映射一段虚拟地址区域
pub fn sys_mmap_tcb(start: usize, len: usize, prot: usize) -> isize {
    TASK_MANAGER.exclusive_access().sys_mmap_tcb(start, len, prot)
}

/// 在当前任务的页表中取消映射一段虚拟地址区域
pub fn sys_munmap_tcb(start: usize, len: usize) -> isize {
    TASK_MANAGER.exclusive_access().sys_munmap_tcb(start, len)
}