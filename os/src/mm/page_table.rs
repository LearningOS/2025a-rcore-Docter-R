//! Implementation of [`PageTableEntry`] and [`PageTable`].
use super::{frame_alloc, FrameTracker, PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    /// bits of page table entry
    pub bits: usize,
}

impl PageTableEntry {
    /// Create a new page table entry
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// Create an empty page table entry
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    /// Get the physical page number from the page table entry
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    /// Get the flags from the page table entry
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// The page pointered by page table entry is valid?
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is readable?
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is writable?
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is executable?
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Create a new page table
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find PageTableEntry by VirtPageNum, create a frame for a 4KB page table if not exist
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find PageTableEntry by VirtPageNum
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// set the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    /// remove the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// get the page table entry from the virtual page number
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// get the physical address from the virtual address
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
    /// get the token from the page table
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// Translate&Copy a ptr[u8] array with LENGTH len to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// Translate&Copy a ptr[u8] array end with `\0` to a `String` Vec through page table
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .get_mut());
        if ch == 0 {
            break;
        }
        string.push(ch as char);
        va += 1;
    }
    string
}

#[allow(unused)]
/// Translate a ptr[u8] array through page table and return a reference of T
pub fn translated_ref<T>(token: usize, ptr: *const T) -> &'static T {
    let page_table = PageTable::from_token(token);
    page_table
        .translate_va(VirtAddr::from(ptr as usize))
        .unwrap()
        .get_ref()
}
/// Translate a ptr[u8] array through page table and return a mutable reference of T
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .get_mut()
}

/// 功能：从起始虚拟地址 `current` 到结束虚拟地址 `ts_va_end`（左闭右开），
/// 借助传入的页表 `page_table` 完成虚拟地址到物理地址的翻译，
/// 并将 `time_val_bytes` 中的数据写入对应物理内存；
/// 自动处理跨页写入场景，拆分数据到对应物理页。
/// 
/// # 参数
/// - `current`: 写入的起始虚拟地址（ usize 类型，需确保在合法地址空间内）
/// - `ts_va_end`: 写入的结束虚拟地址（ usize 类型，不包含此地址，作为范围上限）
/// - `page_table`: 用于地址翻译的页表引用（&PageTable，需提前初始化并包含合法页表项）
/// - `time_val_bytes`: 待写入的原始数据切片（&[u8]，数据长度需匹配地址范围大小，避免越界）
/// 
/// # 返回值
/// - 成功值
/// - 成功：返回 `0`（数据已完整写入对应物理内存）
/// - 失败：返回 `-1`（触发以下任一情况：虚拟地址翻译失败、目标页无写权限）
/// 
/// # 注意事项
/// 1. 依赖 `VirtAddr::floor()`/`page_offset()` 正确拆分虚拟地址，`PageTable::translate()` 正确翻译页表项
/// 2. 依赖 `ppn.get_bytes_array()` 能返回物理页的可变字节切片（内核需提前实现此方法）
pub fn translated_and_write(current: usize, ts_va_end: usize, page_table: &PageTable, time_val_bytes: &[u8]) -> isize {
    let mut data_offset = 0; // 已写入的字节偏移（处理跨页情况）
    let mut current_va = current;
    if current_va >= ts_va_end {
        return -1; // 无数据写入，直接返回失败
    }
    while current_va < ts_va_end {
    // 1 拆分当前虚拟页：获取虚拟页号（VPN）和页内偏移
    let va = VirtAddr::from(current_va);
    let mut vpn = va.floor(); // 当前虚拟页号（向下对齐到页边界）
    let page_offset = va.page_offset(); // 页内偏移（0 ~ 页大小-1）

    // 2 翻译虚拟页到物理页：检查地址有效性和写权限
    let pte = match page_table.translate(vpn) {
        Some(pte) => pte,
        None => return -1, // 虚拟地址无效，返回失败
    };
    if !pte.writable() { // 检查页表项是否有写权限
        return -1; // 无写权限，返回失败
    }
    let ppn = pte.ppn(); // 从页表项中提取物理页号（PPN）

    // 3 计算当前页的写入范围（不超过页边界和结构体结束地址）
    vpn.step(); // 下一个虚拟页号（当前页的结束边界）
    let page_end_va: usize = VirtAddr::from(vpn).into(); // 当前页的结束虚拟地址
    let write_end_va = page_end_va.min(ts_va_end); // 本次写入的结束地址（避免越界）
    let write_len = write_end_va - current_va; // 本次写入的字节数

    // 4 写入物理内存：通过物理页号获取可变切片，复制数据
    let physical_page_slice = ppn.get_bytes_array(); // 物理页的可变字节切片（内核需实现该方法）
    let dest_slice = &mut physical_page_slice[page_offset..page_offset + write_len]; // 目标物理地址范围
    let src_slice = &time_val_bytes[data_offset..data_offset + write_len]; // 待写入的时间数据片段
    dest_slice.copy_from_slice(src_slice); // 复制数据到物理内存

    // 5 更新偏移，处理下一页（若有）
    data_offset += write_len;
    current_va = write_end_va;
}
    0
}

/// An abstraction over a buffer passed from user space to kernel space
pub struct UserBuffer {
    /// A list of buffers
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    /// Constuct UserBuffer
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    /// Get the length of the buffer
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}

/// An iterator over a UserBuffer
pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
