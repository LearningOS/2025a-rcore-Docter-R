//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer, translated_refmut};
use crate::task::{current_task, current_user_token};
use crate::fs::inode::ROOT_INODE;
use crate::fs::OSInode;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    // 通过页表将另一个地址空间中的C风格字符串（以\0结尾的字节数组）转换为Rust的String
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let task = current_task().unwrap();
    let token = current_user_token();
    
    if let Some(file) = task.get_file(fd) {
        // 只处理 OSInode（普通文件）
        if let Some(os_inode) = file.as_any().downcast_ref::<OSInode>() {
            let stat = os_inode.get_stat();
            let st_buf = translated_refmut(token, st);
            *st_buf = stat;
            return 0;
        }
        // 对于其他类型的文件（Stdin/Stdout），暂时返回错误
        // 等普通文件工作后再扩展
        else {
            return -1;
        }
    }
    
    -1
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
        
    let token = current_user_token();
    let old_path = translated_str(token, old_name);
    let new_path = translated_str(token, new_name);
    
    // 检查路径是否相同
    if old_path == new_path {
        return -1;
    }
    
    // 查找原文件
    let old_inode = match ROOT_INODE.find(&old_path) {
        Some(inode) => inode,
        None => return -1, // 原文件不存在
    };
    
    // 检查新文件是否已存在
    if ROOT_INODE.find(&new_path).is_some() {
        return -1; // 新文件已存在
    }
    // 创建硬链接
    match ROOT_INODE.create_hardlink(&new_path, &old_inode) {
        Some(_) => {
            0
        },
        None => {
            -1
        }
    }
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
        
    
    let token = current_user_token();
    let path = translated_str(token, name);
    
    // 检查路径是否为空
    if path.is_empty() {
        return -1;
    }
    
    // 直接调用 ROOT_INODE 的 unlink 方法
    ROOT_INODE.unlink(&path)
}
