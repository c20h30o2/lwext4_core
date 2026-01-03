//! 路径查找

use crate::{
    block::{BlockDev, BlockDevice},
    consts::EXT4_ROOT_INODE,
    error::{Error, ErrorKind, Result},
    inode::Inode,
    superblock::Superblock,
};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::entry::DirIterator;

/// 路径查找器
///
/// 用于根据路径字符串查找 inode
pub struct PathLookup<'a, D: BlockDevice> {
    bdev: &'a mut BlockDev<D>,
    sb: &'a Superblock,
}

impl<'a, D: BlockDevice> PathLookup<'a, D> {
    /// 创建新的路径查找器
    pub fn new(bdev: &'a mut BlockDev<D>, sb: &'a Superblock) -> Self {
        Self { bdev, sb }
    }

    /// 在目录中查找指定名称的条目
    ///
    /// # 参数
    ///
    /// * `dir_inode` - 目录 inode
    /// * `name` - 要查找的文件名
    ///
    /// # 返回
    ///
    /// 找到的 inode 编号，如果不存在返回 None
    fn lookup_in_dir(&mut self, dir_inode: &Inode, name: &str) -> Result<Option<u32>> {
        let mut iter = DirIterator::new(self.bdev, self.sb, dir_inode)?;

        while let Some(entry) = iter.next_entry()? {
            if entry.name == name {
                return Ok(Some(entry.inode));
            }
        }

        Ok(None)
    }

    /// 根据路径查找 inode
    ///
    /// # 参数
    ///
    /// * `path` - 路径字符串（绝对路径或相对路径）
    ///
    /// # 返回
    ///
    /// 找到的 inode 编号
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let inode_num = lookup.find_inode("/bin/ls")?;
    /// ```
    pub fn find_inode(&mut self, path: &str) -> Result<u32> {
        if path.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "Empty path"));
        }

        // 分割路径
        let components: Vec<String> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        if components.is_empty() {
            // 只有 "/" 的情况
            return Ok(EXT4_ROOT_INODE);
        }

        // 从根目录开始
        let mut current_inode_num = EXT4_ROOT_INODE;

        for component in &components {
            // 跳过空组件和 "."
            if component.is_empty() || component == "." {
                continue;
            }

            // 处理 ".."
            if component == ".." {
                // TODO: 实现 ".." 处理（需要记录父目录）
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    ".. not yet supported",
                ));
            }

            // 读取当前 inode
            let current_inode = Inode::load(self.bdev, self.sb, current_inode_num)?;

            // 确保当前 inode 是目录
            if !current_inode.is_dir() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Not a directory",
                ));
            }

            // 在目录中查找下一个组件
            match self.lookup_in_dir(&current_inode, component.as_str())? {
                Some(inode_num) => {
                    current_inode_num = inode_num;
                }
                None => {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        "Path component not found",
                    ));
                }
            }
        }

        Ok(current_inode_num)
    }

    /// 根据路径读取 inode
    ///
    /// # 参数
    ///
    /// * `path` - 路径字符串
    ///
    /// # 返回
    ///
    /// Inode 对象
    pub fn get_inode(&mut self, path: &str) -> Result<Inode> {
        let inode_num = self.find_inode(path)?;
        Inode::load(self.bdev, self.sb, inode_num)
    }
}

/// 便捷函数：根据路径查找 inode 编号
///
/// # 参数
///
/// * `bdev` - 块设备引用
/// * `sb` - superblock 引用
/// * `path` - 路径字符串
pub fn lookup_path<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    sb: &Superblock,
    path: &str,
) -> Result<u32> {
    let mut lookup = PathLookup::new(bdev, sb);
    lookup.find_inode(path)
}

/// 便捷函数：根据路径读取 inode
///
/// # 参数
///
/// * `bdev` - 块设备引用
/// * `sb` - superblock 引用
/// * `path` - 路径字符串
pub fn get_inode_by_path<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    sb: &Superblock,
    path: &str,
) -> Result<Inode> {
    let mut lookup = PathLookup::new(bdev, sb);
    lookup.get_inode(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_lookup_api() {
        // 这些测试需要实际的块设备和 ext4 文件系统
        // 主要是验证 API 的设计和编译
    }
}
