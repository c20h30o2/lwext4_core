//! 目录读取器 - ArceOS 兼容接口
//!
//! 这个模块提供了与 ArceOS VFS 兼容的目录读取 API。
//! 它在底层的 DirIterator 上提供了一个有状态的包装器。
//!
//! ## 设计说明
//!
//! - **有状态**: 与 DirIterator 不同，DirReader 持有 InodeRef 引用和当前条目
//! - **ArceOS 兼容**: 提供 current() + step() API，而不是 Iterator 风格
//! - **延迟加载**: 在 new() 时加载第一个条目，在 step() 时加载下一个
//!
//! ## 与 DirIterator 的关系
//!
//! DirIterator 是底层的、无状态的迭代器。
//! DirReader 是上层的、有状态的包装器，为 ArceOS 集成提供友好的 API。

use crate::{
    block::BlockDevice,
    error::Result,
    fs::InodeRef,
};

use super::iterator::{DirEntry, DirIterator};

/// 目录读取器
///
/// 为 ArceOS VFS 提供兼容的目录读取接口
///
/// ## 使用示例
///
/// ```ignore
/// let mut reader = DirReader::new(&mut inode_ref, 0)?;
///
/// while let Some(entry) = reader.current() {
///     println!("Found: {} (inode {})", entry.name, entry.inode);
///     reader.step()?;
/// }
/// ```
pub struct DirReader<'a, 'b, D: BlockDevice> {
    /// 底层迭代器
    iter: DirIterator,
    /// 目录的 inode 引用
    inode_ref: &'a mut InodeRef<'b, D>,
    /// 当前目录项（缓存）
    current_entry: Option<DirEntry>,
}

impl<'a, 'b, D: BlockDevice> DirReader<'a, 'b, D> {
    /// 创建新的目录读取器
    ///
    /// # 参数
    ///
    /// * `inode_ref` - 目录的 inode 引用
    /// * `offset` - 起始位置（字节偏移）
    ///
    /// # 返回
    ///
    /// - `Ok(DirReader)` - 成功创建，并加载了第一个条目（如果存在）
    /// - `Err(_)` - 创建失败或读取第一个条目失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 从头开始读取
    /// let mut reader = DirReader::new(&mut inode_ref, 0)?;
    ///
    /// // 从特定偏移开始读取
    /// let mut reader = DirReader::new(&mut inode_ref, 1024)?;
    /// ```
    pub fn new(inode_ref: &'a mut InodeRef<'b, D>, offset: u64) -> Result<Self> {
        let mut iter = DirIterator::new(inode_ref, offset)?;

        // 读取第一个条目
        let current_entry = iter.next(inode_ref)?;

        Ok(Self {
            iter,
            inode_ref,
            current_entry,
        })
    }

    /// 获取当前目录项
    ///
    /// # 返回
    ///
    /// - `Some(&DirEntry)` - 当前有效的目录项
    /// - `None` - 已到达目录末尾
    ///
    /// 注意：此方法不会推进迭代器，多次调用返回同一条目
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if let Some(entry) = reader.current() {
    ///     println!("Current entry: {}", entry.name);
    ///     // 可以多次访问同一条目
    ///     println!("Again: {}", entry.name);
    /// }
    /// ```
    pub fn current(&self) -> Option<&DirEntry> {
        self.current_entry.as_ref()
    }

    /// 推进到下一个目录项
    ///
    /// # 返回
    ///
    /// - `Ok(())` - 成功推进（可能到达末尾，current() 返回 None）
    /// - `Err(_)` - 读取下一个条目时发生错误
    ///
    /// # 行为
    ///
    /// - 如果已在末尾，调用 step() 不会产生错误，但 current() 仍返回 None
    /// - 成功后，current() 会返回新的条目（如果有）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// while let Some(entry) = reader.current() {
    ///     println!("{}", entry.name);
    ///     reader.step()?; // 推进到下一个
    /// }
    /// ```
    pub fn step(&mut self) -> Result<()> {
        // 读取下一个条目
        self.current_entry = self.iter.next(self.inode_ref)?;
        Ok(())
    }

    /// 获取当前文件偏移
    ///
    /// # 返回
    ///
    /// 当前位置的字节偏移
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let offset = reader.offset();
    /// println!("Current position: {}", offset);
    /// ```
    pub fn offset(&self) -> u64 {
        self.iter.current_offset()
    }

    /// 定位到指定偏移
    ///
    /// # 参数
    ///
    /// * `offset` - 目标位置（字节偏移）
    ///
    /// # 返回
    ///
    /// - `Ok(())` - 成功定位并加载了该位置的条目
    /// - `Err(_)` - 定位失败或读取条目失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// reader.seek(1024)?;
    /// if let Some(entry) = reader.current() {
    ///     println!("Entry at offset 1024: {}", entry.name);
    /// }
    /// ```
    pub fn seek(&mut self, offset: u64) -> Result<()> {
        self.iter.seek(self.inode_ref, offset)?;

        // 重新加载当前条目
        self.current_entry = self.iter.next(self.inode_ref)?;

        Ok(())
    }

    /// 检查是否到达目录末尾
    ///
    /// # 返回
    ///
    /// - `true` - 已到达末尾，current() 返回 None
    /// - `false` - 还有更多条目
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if reader.is_at_end() {
    ///     println!("No more entries");
    /// }
    /// ```
    pub fn is_at_end(&self) -> bool {
        self.current_entry.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_reader_api() {
        // 这个测试只验证 API 签名编译通过
        // 实际功能测试需要完整的文件系统环境

        // 验证 DirReader 的泛型参数正确
        fn _check_reader_signature<'a, 'b, D: BlockDevice>(
            inode_ref: &'a mut InodeRef<'b, D>,
        ) -> Result<DirReader<'a, 'b, D>> {
            DirReader::new(inode_ref, 0)
        }
    }

    #[test]
    fn test_dir_entry_reexport() {
        // 验证 DirEntry 可以正常使用
        let entry = DirEntry {
            inode: 2,
            name: "test".into(),
            file_type: 1,
        };

        assert_eq!(entry.inode, 2);
        assert_eq!(entry.name, "test");
    }
}
