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

    /// 选择 stride 最小的任务进行调度
    pub fn fetch_stride(&mut self) -> Option<Arc<TaskControlBlock>> {
    if self.ready_queue.is_empty() {
        None
    } else {
        // 找到 stride 最小的任务
        let mut min_stride = usize::MAX;
        let mut min_index = 0;
        
        for (i, task) in self.ready_queue.iter().enumerate() {
            let inner = task.inner_exclusive_access();
            if inner.stride < min_stride {
                min_stride = inner.stride;
                min_index = i;
            }
        }
        
        // 移除选中的任务
        let task = self.ready_queue.remove(min_index).unwrap();
        
        // 更新被选中任务的 stride
        {
            let pass = {
                let inner = task.inner_exclusive_access();
                inner.pass
            }; // 这里 inner 被释放

            let mut inner = task.inner_exclusive_access();
            // 检查溢出
            if let Some(new_stride) = inner.stride.checked_add(pass) {
                inner.stride = new_stride;
            } else {
                // 处理溢出：归一化所有任务的 stride
                inner.stride = pass;
                self.normalize_strides();
                }
            } // 这里 inner 被释放

        Some(task)
        }
    }

    /// 归一化所有任务的 stride 值，避免溢出
    fn normalize_strides(&mut self) {
        if self.ready_queue.is_empty() {
            return;
        }
        
        // 找到最小的 stride
        let min_stride = self.ready_queue.iter()
            .map(|task| task.inner_exclusive_access().stride)
            .min()
            .unwrap();
        
        // 所有任务减去最小 stride
        for task in self.ready_queue.iter_mut() {
            let mut inner = task.inner_exclusive_access();
            inner.stride = inner.stride.saturating_sub(min_stride);
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