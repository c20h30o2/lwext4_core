//! 文件句柄

use crate::{
    block::{BlockDev, BlockDevice},
    error::{Error, ErrorKind, Result},
    extent::ExtentTree,
    superblock::Superblock,
};

use super::filesystem::Ext4FileSystem;

/// 文件句柄
///
/// 表示一个打开的文件，支持读取和定位操作
///
/// # 设计说明
///
/// 与旧设计不同，File 不再持有 inode 数据的副本，而是只保存 inode 编号。
/// 每次需要访问 inode 数据时，都从文件系统临时获取最新数据，确保一致性。
///
/// 这种设计的优点：
/// - **数据一致性**: 总是访问最新的 inode 数据
/// - **内存效率**: 不复制 ~160 字节的 inode 结构
/// - **与 lwext4 一致**: lwext4 的 ext4_file 也不持有 inode 数据
pub struct File<D: BlockDevice> {
    /// Inode 编号
    inode_num: u32,
    /// 当前文件偏移
    offset: u64,
    /// 块大小（缓存以提高性能）
    block_size: u32,
    _phantom: core::marker::PhantomData<D>,
}

impl<D: BlockDevice> File<D> {
    /// 创建新的文件句柄（内部使用）
    pub(super) fn new(
        _bdev: &mut BlockDev<D>,
        sb: &Superblock,
        inode_num: u32,
    ) -> Result<Self> {
        Ok(Self {
            inode_num,
            offset: 0,
            block_size: sb.block_size(),
            _phantom: core::marker::PhantomData,
        })
    }

    /// 读取文件内容
    ///
    /// 从当前位置读取数据到缓冲区，并更新文件位置
    ///
    /// # 参数
    ///
    /// * `fs` - 文件系统引用
    /// * `buf` - 目标缓冲区
    ///
    /// # 返回
    ///
    /// 实际读取的字节数（可能小于缓冲区大小）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut file = fs.open("/etc/passwd")?;
    /// let mut buf = vec![0u8; 1024];
    /// let n = file.read(&mut fs, &mut buf)?;
    /// println!("Read {} bytes", n);
    /// ```
    pub fn read(&mut self, fs: &mut Ext4FileSystem<D>, buf: &mut [u8]) -> Result<usize> {
        // ✅ 使用 InodeRef 的辅助方法，保证数据一致性
        let mut inode_ref = fs.get_inode_ref(self.inode_num)?;

        // 检查 EOF
        let file_size = inode_ref.size()?;
        if self.offset >= file_size {
            return Ok(0); // EOF
        }

        let n = inode_ref.read_extent_file(self.offset, buf)?;
        self.offset += n as u64;

        Ok(n)
    }

    /// 读取整个文件内容
    ///
    /// # 参数
    ///
    /// * `fs` - 文件系统引用
    ///
    /// # 返回
    ///
    /// 文件内容（Vec<u8>）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut file = fs.open("/etc/passwd")?;
    /// let content = file.read_to_end(&mut fs)?;
    /// let text = String::from_utf8_lossy(&content);
    /// ```
    pub fn read_to_end(&mut self, fs: &mut Ext4FileSystem<D>) -> Result<alloc::vec::Vec<u8>> {
        // 获取文件大小
        let file_size = self.size(fs)?;

        if file_size > usize::MAX as u64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "File too large to read into memory",
            ));
        }

        let mut buf = alloc::vec![0u8; file_size as usize];
        let mut total_read = 0;

        while total_read < buf.len() {
            let n = self.read(fs, &mut buf[total_read..])?;
            if n == 0 {
                break; // EOF
            }
            total_read += n;
        }

        buf.truncate(total_read);
        Ok(buf)
    }

    /// 移动文件指针
    ///
    /// # 参数
    ///
    /// * `fs` - 文件系统引用
    /// * `pos` - 新的位置（字节偏移）
    ///
    /// # 返回
    ///
    /// 新的位置
    ///
    /// # 注意
    ///
    /// 允许 seek 到文件末尾之后，实际读取时会返回 EOF
    pub fn seek(&mut self, fs: &mut Ext4FileSystem<D>, pos: u64) -> Result<u64> {
        // 获取文件大小用于验证（可选）
        let file_size = self.size(fs)?;

        // 允许 seek 到文件大小，但警告超出范围
        if pos > file_size {
            // 不返回错误，允许 seek 超过文件末尾
            // 读取时会返回 EOF
        }

        self.offset = pos;
        Ok(self.offset)
    }

    /// 获取当前文件指针位置
    pub fn position(&self) -> u64 {
        self.offset
    }

    /// 获取文件大小
    ///
    /// # 参数
    ///
    /// * `fs` - 文件系统引用
    pub fn size(&self, fs: &mut Ext4FileSystem<D>) -> Result<u64> {
        let mut inode_ref = fs.get_inode_ref(self.inode_num)?;
        inode_ref.size()
    }

    /// 获取 inode 编号
    pub fn inode_num(&self) -> u32 {
        self.inode_num
    }

    /// 重置文件指针到起始位置
    pub fn rewind(&mut self) {
        self.offset = 0;
    }

    // ========== 写操作 ==========

    /// 写入数据到文件
    ///
    /// 从当前位置写入数据，并更新文件位置
    ///
    /// # 参数
    ///
    /// * `fs` - 文件系统引用
    /// * `buf` - 要写入的数据
    ///
    /// # 返回
    ///
    /// 实际写入的字节数
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut file = fs.open("/tmp/test.txt")?;
    /// let n = file.write(&mut fs, b"Hello, World!")?;
    /// println!("Wrote {} bytes", n);
    /// ```
    pub fn write(&mut self, fs: &mut Ext4FileSystem<D>, buf: &[u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // 🚀 性能优化：使用批量写入接口，一次性处理所有数据
        // 相比单块写入，避免了多次 InodeRef 获取/释放
        let write_len = fs.write_at_inode_batch(self.inode_num, buf, self.offset)?;

        // 更新文件位置
        self.offset += write_len as u64;

        Ok(write_len)
    }

    /// 截断文件到指定大小
    ///
    /// # 参数
    ///
    /// * `fs` - 文件系统引用
    /// * `size` - 新的文件大小
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut file = fs.open("/tmp/test.txt")?;
    /// file.truncate(&mut fs, 100)?; // 截断到 100 字节
    /// ```
    pub fn truncate(&mut self, fs: &mut Ext4FileSystem<D>, size: u64) -> Result<()> {
        // 调用文件系统级别的 truncate
        fs.truncate_file(self.inode_num, size)?;

        // 如果当前 offset 超过了新大小，调整到文件末尾
        if self.offset > size {
            self.offset = size;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_api() {
        // 这些测试需要实际的块设备和 ext4 文件系统
        // 主要是验证 API 的设计和编译
    }
}
