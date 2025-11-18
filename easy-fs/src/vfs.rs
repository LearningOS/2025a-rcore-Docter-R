use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::format;
use crate::alloc::string::ToString;
use spin::{Mutex, MutexGuard};

/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    pub fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            assert!(root_inode.is_dir());
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        
        // 创建新文件
        let new_inode_id = fs.alloc_inode();
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
                // nlink 已经在 initialize 中设置为1
            });

        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            self.increase_size(new_size as u32, root_inode, &mut fs);
            
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }

    /// 获得当前 inode 的 inode_id
    pub fn get_inode_id(&self) -> u32 {
        let fs = self.fs.lock();
        fs.get_inode_id(self.block_id as u32, self.block_offset)
    }

    /// 判断当前 inode 是否为目录
    pub fn is_dir(&self) -> bool {
        self.read_disk_inode(|disk_inode| disk_inode.is_dir())
    }

    /// 判断当前 inode 是否为文件  
    pub fn is_file(&self) -> bool {
        self.read_disk_inode(|disk_inode| disk_inode.is_file())
    }

    /// 增加硬链接计数
    pub fn inc_nlink(&self) {
        self.modify_disk_inode(|disk_inode| {
            disk_inode.nlink += 1;
        });
        block_cache_sync_all();
    }

    /// 减少硬链接计数
    pub fn dec_nlink(&self) {
        self.modify_disk_inode(|disk_inode| {
            disk_inode.nlink -= 1;
        });
        block_cache_sync_all();
    }

    /// 获取硬链接计数
    pub fn get_nlink(&self) -> u32 {
        self.read_disk_inode(|disk_inode| disk_inode.nlink)
    }

    /// 在当前目录下添加目录项
    pub fn add_dir_entry(&self, name: &str, inode_id: u32) -> Option<()> {
        let mut fs = self.fs.lock();
        
        self.modify_disk_inode(|disk_inode| {
            // 确保是目录
            if !disk_inode.is_dir() {
                return None;
            }

            // 检查是否已存在同名条目
            if self.find_inode_id(name, disk_inode).is_some() {
                return None;
            }

            // 计算新的目录大小
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;

            // 增加目录大小
            self.increase_size(new_size as u32, disk_inode, &mut fs);

            // 创建新的目录项
            let dirent = DirEntry::new(name, inode_id);

            // 在目录末尾写入新的目录项
            disk_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );

            Some(())
        })
    }

    /// 解析路径，返回父目录路径和文件名
    pub fn parse_path(path: &str) -> Option<(String, String)> {
    // 去除路径开头和结尾的所有 / 字符
    let path = path.trim_matches('/');
    if path.is_empty() {
        return None;
    }
    
    // rfind('/') 方法是从字符串的右侧开始向左搜索直到遇到的一个'/'停止（也就是从左往右最后一个"\"），
    // 但是返回的索引位置仍然是相对于字符串的起始位置（即从左数起的索引）。
    if let Some(last_slash) = path.rfind('/') {
        // 由于 trim_matches('/')，last_slash 不可能为 0
        let parent_path = format!("/{}", &path[..last_slash]);
        let filename = &path[last_slash + 1..];
        
        if filename.is_empty() {
            return None;
        }
        Some((parent_path, filename.to_string()))
    } else {
        // 没有找到 /，说明文件在根目录下
        Some(("/".to_string(), path.to_string()))
    }
}

    /// 获取文件大小（公共方法）
    pub fn get_size(&self) -> u64 {
        self.read_disk_inode(|disk_inode| disk_inode.size as u64)
    }

    /// 创建目录
    pub fn create_dir(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        
        // 检查是否已存在
        let op = |root_inode: &DiskInode| {
            assert!(root_inode.is_dir());
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        
        // 创建新目录
        let new_inode_id = fs.alloc_inode();
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::Directory);
                new_inode.nlink = 2; // 目录初始链接数为2 (自身和 ".")
            });

        // 在父目录中添加目录项
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            self.increase_size(new_size as u32, root_inode, &mut fs);
            
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        
        // 在新目录中添加 "." 和 ".." 条目
        let new_dir_inode = Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        ));
        
        // 添加 "." 条目（指向自己）
        new_dir_inode.modify_disk_inode(|disk_inode| {
            let mut size = 0;
            
            // 添加 "."
            let self_dirent = DirEntry::new(".", new_inode_id);
            disk_inode.write_at(size, self_dirent.as_bytes(), &self.block_device);
            size += DIRENT_SZ;
            
            // 添加 ".."（指向父目录）
            let parent_inode_id = self.get_inode_id();
            let parent_dirent = DirEntry::new("..", parent_inode_id);
            disk_inode.write_at(size, parent_dirent.as_bytes(), &self.block_device);
            size += DIRENT_SZ;
            
            disk_inode.size = size as u32;
        });

        block_cache_sync_all();
        Some(new_dir_inode)
    }

    /// 创建硬链接
    pub fn create_hardlink(&self, new_path: &str, old_inode: &Arc<Inode>) -> Option<Arc<Inode>> { 
        // 对于根目录下的文件，直接使用当前目录（根目录）作为父目录
        let parent_inode = self; // 根目录自身作为父目录
        let filename = new_path; // 整个路径就是文件名

        // 确保父目录确实是目录
        if !parent_inode.is_dir() {
            return None;
        }

        // 检查新文件名是否已存在
        if parent_inode.find(filename).is_some() {
            return None;
        }

        // 不能为目录创建硬链接
        if old_inode.is_dir() {
            return None;
        }

        // 获取原文件的 inode_id
        let old_inode_id = old_inode.get_inode_id();

        // 在父目录（根目录）中添加新的目录项
        match parent_inode.add_dir_entry(filename, old_inode_id) {
            Some(()) => {
                ();
            },
            None => {
                return None;
            }
        }
        // 增加原文件的硬链接计数
        old_inode.inc_nlink();
        Some(old_inode.clone())
    }

    /// 删除指定路径的文件（unlink 功能）
    pub fn unlink(&self, path: &str) -> isize {
        // 对于根目录下的文件，直接使用当前目录（根目录）作为父目录
        let parent_inode = self; // 根目录自身作为父目录
        let filename = path; // 整个路径就是文件名
        
        // 查找要删除的文件
        let target_inode = match parent_inode.find(filename) {
            Some(inode) => inode,
            None => {
                return -1;
            }
        };
        
        // 检查是否是目录（不允许删除目录）
        if target_inode.is_dir() {
            return -1;
        }
        
        // 从父目录（根目录）中删除目录项
        if parent_inode.remove_dir_entry(filename).is_none() {
            return -1;
        }
        
        // 减少硬链接计数
        let before_nlink = target_inode.get_nlink();
        target_inode.dec_nlink();
        let after_nlink = target_inode.get_nlink();
        
        // 如果链接数为0，回收资源
        if after_nlink == 0 {
            target_inode.clear();
            // 注意：这里还应该释放 inode，但当前 EasyFileSystem 没有释放 inode 的机制
        }
        
        0
    }

    /// 从目录中删除指定名称的目录项
    pub fn remove_dir_entry(&self, name: &str) -> Option<()> {
        let _fs = self.fs.lock();
        
        self.modify_disk_inode(|disk_inode| {
            // 确保是目录
            if !disk_inode.is_dir() {
                return None;
            }

            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut found_index = None;
            let mut target_dirent = DirEntry::empty();

            // 查找要删除的目录项
            for i in 0..file_count {
                assert_eq!(
                    disk_inode.read_at(DIRENT_SZ * i, target_dirent.as_bytes_mut(), &self.block_device),
                    DIRENT_SZ,
                );
                if target_dirent.name() == name {
                    found_index = Some(i);
                    break;
                }
            }

            if let Some(index) = found_index {
                // 如果是最后一个条目，直接截断
                if index == file_count - 1 {
                    disk_inode.size -= DIRENT_SZ as u32;
                } else {
                    // 用最后一个条目覆盖要删除的条目
                    let mut last_dirent = DirEntry::empty();
                    let last_index = file_count - 1;
                    assert_eq!(
                        disk_inode.read_at(
                            DIRENT_SZ * last_index,
                            last_dirent.as_bytes_mut(),
                            &self.block_device
                        ),
                        DIRENT_SZ,
                    );
                    
                    // 写入最后一个条目到被删除的位置
                    disk_inode.write_at(
                        DIRENT_SZ * index,
                        last_dirent.as_bytes(),
                        &self.block_device,
                    );
                    
                    // 截断目录大小
                    disk_inode.size -= DIRENT_SZ as u32;
                }
                Some(())
            } else {
                None
            }
        })
    }

}
