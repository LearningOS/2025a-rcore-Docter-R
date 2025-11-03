//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,current_user_token,get_current_syscall_stats};
use crate::timer::get_time_us;
use crate::mm::{PageTable,translated_and_write,VirtAddr,frame_alloc,PTEFlags};
use crate::config::{PAGE_SIZE,USER_SPACE_END};
use alloc::vec::Vec;

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
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    // 1. 为内核空间获取当前任务的根页表
    let page_table_token = current_user_token();
    let page_table = PageTable::from_token(page_table_token);

    // 2. 将用户态指针转为虚拟地址（usize）
    let id_start = id; 
    let va = VirtAddr::from(id_start);
    let vpn = va.floor(); // 当前虚拟页号（向下对齐到页边界）
    let page_offset = va.page_offset(); // 页内偏移（0 ~ 页大小-1）


    match trace_request {
        0 => {
            // set mask
            // 翻译虚拟页到物理页：检查地址有效性和写权限
            let pte = match page_table.translate(vpn) {
                Some(pte) => pte,
                None => return -1, // 虚拟地址无效，返回失败
            };
            if !pte.readable() { // 检查页表项是否有读权限
                return -1; // 无写权限，返回失败
            }
            if !pte.user_visible() { // 检查页表项是否对用户可见
                return -1; // 不可见，返回失败
            }
            let ppn = pte.ppn(); // 从页表项中提取物理页号（PPN）
            ppn.get_bytes_array()[page_offset] as isize
        }
        1 => {
            // get mask
            // 翻译虚拟页到物理页：检查地址有效性和写权限
            let pte = match page_table.translate(vpn) {
                Some(pte) => pte,
                None => return -1, // 虚拟地址无效，返回失败
            };
            if !pte.writable() { // 检查页表项是否有写权限
                return -1; // 无写权限，返回失败
            }
            if !pte.user_visible() { // 检查页表项是否对用户可见
                return -1; // 不可见，返回失败
            }

            let ppn = pte.ppn(); // 从页表项中提取物理页号（PPN）
            ppn.get_bytes_array()[page_offset] = data as u8;
            0 as isize
        }
        2 => {
            // set name
            get_current_syscall_stats().get(&id).cloned().unwrap_or(0) as isize
            
        }
        _ => -1 as isize,
    }
}

pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    
    // 1. 校验start按页对齐（页大小为PAGE_SIZE，4KB为4096）
    if start % PAGE_SIZE != 0 {
        return -1;
    }

    // 2. 校验prot合法性：仅低3位有效，且至少有一个权限位
    if (prot & !0x7) != 0 || (prot & 0x7) == 0 {
        return -1;
    }

    // 3. 计算映射区间[start, end)，处理len=0的特殊情况
    let end = start.checked_add(len).unwrap_or(usize::MAX);
    // 校验区间不超出用户空间（避免映射内核地址）
    if start >= USER_SPACE_END || end > USER_SPACE_END {
        return -1;
    }

    // 4. 计算需要映射的页数（向上取整）
    if len == 0 {
        return 0;
    }

    // 5. 获取当前进程的页表
    let page_table_token = current_user_token();
    let mut page_table = PageTable::from_token(page_table_token);

    let mut flags = PTEFlags::V | PTEFlags::U; // 基础标志：有效+用户可见
    if (prot & 0x1) != 0 { flags |= PTEFlags::R; } // prot第0位→读权限
    if (prot & 0x2) != 0 { flags |= PTEFlags::W; } // prot第1位→写权限
    if (prot & 0x4) != 0 { flags |= PTEFlags::X; } // prot第2位→执行权限

    // 6. 检查区间内是否已有映射（如有则返回错误）
    let mut va = start;
    while va < end {
        let vpn = VirtAddr::from(va).floor(); // 虚拟页号（向下取整到页边界）
        
        if let Some(pte) = page_table.translate(vpn) {
            // 判断页面是否有效（检查PTE的有效位V是否置位）
            if pte.is_valid() {
                return -1; // 存在已有效的映射页，冲突返回-1
            }
        }

        va += PAGE_SIZE;
    }

    // 7. 建立映射，处理内存分配失败
    let mut va = start;
    let mut mapped_pages = Vec::new(); // 记录已成功映射的页面，用于错误回滚
    
    while va < end {
        let vpn = VirtAddr::from(va).floor();
        
        // 分配物理页（处理内存不足情况）
        if let Some(frame) = frame_alloc() {
            let ppn = frame.ppn;
            
            // 建立虚拟页→物理页的映射
            page_table.map(vpn, ppn, flags);
            mapped_pages.push(vpn); // 记录成功映射的页面
        } else {
            // 内存不足，回滚已建立的映射
            for mapped_vpn in mapped_pages {
                page_table.unmap(mapped_vpn);
            }
            return -1;
        }

        va += PAGE_SIZE;
    }

    0 // 成功返回0
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
        
    // 1. 校验start按页对齐（虚拟内存最小单位是页）
    if start % PAGE_SIZE != 0 {
        return -1;
    }

    // 2. len=0时无实际操作，直接返回成功
    if len == 0 {
        return 0;
    }

    // 3. 计算映射区间[start, end)
    let end = start.checked_add(len).unwrap_or(usize::MAX);
    // 校验区间：不超出用户空间
    if start > end || end > USER_SPACE_END {
        return -1;
    }

    // 4. 获取当前进程的页表
    let page_table_token = current_user_token();
    let mut page_table = PageTable::from_token(page_table_token);

    // 5. 首先检查整个区域是否都是已映射的
    // 如果任何一页没有有效映射，则返回错误
    let mut va = start;
    while va < end {
        let vpn = VirtAddr::from(va).floor();
        
        // 检查页面是否有效映射
        if let Some(pte) = page_table.translate(vpn) {
            if !pte.is_valid() || !pte.user_visible() {
                // 页面无效，返回错误
                return -1;
            }else {
                page_table.unmap(vpn);
            }
        } 
        va += PAGE_SIZE;
    }


    // 7. 刷新TLB：确保CPU立即丢弃旧映射
    unsafe {
        core::arch::asm!("sfence.vma");
    }

    0 // 成功返回0
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
