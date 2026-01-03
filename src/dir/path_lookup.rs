//! 路径查找
//!
//! 对应 lwext4 的路径解析相关功能
//!
//! ## 设计改进
//!
//! 与旧的 `lookup.rs` 实现相比，新设计：
//! 1. **使用 InodeRef** - 而不是 Inode::load() 加载拷贝
//! 2. **使用新的 DirIterator** - 基于 Block handle 的迭代器
//! 3. **更好的错误处理** - 区分不同类型的错误

use crate::{
    block::{BlockDev, BlockDevice},
    consts::EXT4_ROOT_INODE,
    error::{Error, ErrorKind, Result},
    fs::InodeRef,
    superblock::Superblock,
};
use alloc::{string::{String, ToString}, vec::Vec};

use super::iterator::DirIterator;

/// 路径查找器
///
/// 用于根据路径字符串查找 inode
pub struct PathLookup<'a, D: BlockDevice> {
    bdev: &'a mut BlockDev<D>,
    sb: &'a mut Superblock,
}

impl<'a, D: BlockDevice> PathLookup<'a, D> {
    /// 创建新的路径查找器
    pub fn new(bdev: &'a mut BlockDev<D>, sb: &'a mut Superblock) -> Self {
        Self { bdev, sb }
    }


    /// 根据路径查找 inode
    ///
    /// 对应 lwext4 的 `ext4_path2inode()`
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
    /// let mut lookup = PathLookup::new(&mut bdev, &sb);
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
                // TODO: 实现 ".." 处理（需要记录父目录或读取 ".." 条目）
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    ".. not yet supported",
                ));
            }

            // 获取当前 inode 的引用
            let mut current_inode_ref = InodeRef::get(self.bdev, self.sb, current_inode_num)?;

            // 确保当前 inode 是目录
            if !current_inode_ref.is_dir()? {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Not a directory",
                ));
            }

            // 在目录中查找下一个组件
            let mut iter = DirIterator::new(&mut current_inode_ref, 0)?;
            let mut found_inode = None;

            while let Some(entry) = iter.next(&mut current_inode_ref)? {
                if entry.name == component.as_str() {
                    found_inode = Some(entry.inode);
                    break;
                }
            }

            match found_inode {
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

            // current_inode_ref 在此处自动释放（Drop）
        }

        Ok(current_inode_num)
    }

    /// 根据路径获取 InodeRef
    ///
    /// # 参数
    ///
    /// * `path` - 路径字符串
    ///
    /// # 返回
    ///
    /// InodeRef 对象
    ///
    /// # 注意
    ///
    /// 返回的 InodeRef 持有 BlockDev 的可变引用，
    /// 因此在 InodeRef 存在期间无法使用 PathLookup。
    pub fn get_inode_ref(&mut self, path: &str) -> Result<InodeRef<D>> {
        let inode_num = self.find_inode(path)?;
        InodeRef::get(self.bdev, self.sb, inode_num)
    }
}

/// 便捷函数：根据路径查找 inode 编号
///
/// # 参数
///
/// * `bdev` - 块设备引用
/// * `sb` - superblock 引用（可变）
/// * `path` - 路径字符串
pub fn lookup_path<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    sb: &mut Superblock,
    path: &str,
) -> Result<u32> {
    let mut lookup = PathLookup::new(bdev, sb);
    lookup.find_inode(path)
}

/// 便捷函数：根据路径获取 InodeRef
///
/// # 参数
///
/// * `bdev` - 块设备引用
/// * `sb` - superblock 引用（可变）
/// * `path` - 路径字符串
pub fn get_inode_ref_by_path<'a, D: BlockDevice>(
    bdev: &'a mut BlockDev<D>,
    sb: &'a mut Superblock,
    path: &str,
) -> Result<InodeRef<'a, D>> {
    // 先查找 inode 编号
    let inode_num = lookup_path(bdev, sb, path)?;
    // 然后使用原始的 bdev 和 sb 引用创建 InodeRef
    InodeRef::get(bdev, sb, inode_num)
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
