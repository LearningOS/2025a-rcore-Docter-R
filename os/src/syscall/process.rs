//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,current_user_token};
use crate::timer::get_time_us;
use crate::mm::{PageTable,translated_and_write};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // 1. 为内核空间获取当前任务的根页表
    let page_table_token = current_user_token();
    let page_table = PageTable::from_token(page_table_token);

    // 2. 将用户态指针转为虚拟地址（usize），计算 TimeVal 结构体的地址范围
    let ts_va_start = ts as usize; // TimeVal 起始虚拟地址
    let ts_va_end = ts_va_start + core::mem::size_of::<TimeVal>(); // 结束虚拟地址（含结构体大小）
    let current_va = ts_va_start;

    // 3. 读取硬件时间
    let us = get_time_us();
    let (sec, usec) = (us / 1_000_000, us % 1_000_000);
    let time_val = TimeVal { sec, usec };
    // 将 TimeVal 转为字节数组，方便后续写入物理内存
    let time_val_bytes = unsafe { core::slice::from_raw_parts(&time_val as *const _ as *const u8, core::mem::size_of::<TimeVal>()) };
    

    // 4. 遍历 TimeVal 地址范围，按页翻译并写入数据（处理跨页，虽结构体小但兼容通用情况）
    translated_and_write(current_va, ts_va_end, &page_table, time_val_bytes)
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    -1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
