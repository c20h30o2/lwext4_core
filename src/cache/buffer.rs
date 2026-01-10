//! 缓存块结构
//!
//! 对应 lwext4 的 `ext4_buf` 结构
//!
//! 🔧 重构说明：使用 lru crate 后大幅简化
//! - 删除引用计数（refctr）：lru crate 自动管理生命周期
//! - 删除LRU ID（lru_id）：lru crate 内部维护访问顺序
//! - 删除块ID（id）：直接使用 lba 作为key

use crate::error::Result;
use alloc::boxed::Box;
use alloc::vec::Vec;
use bitflags::bitflags;

bitflags! {
    /// 缓存块标志
    ///
    /// 对应 lwext4 的 `EXT4_BCACHE_FLAG_*` 常量
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CacheFlags: u8 {
        /// 数据已更新（有效）
        const UPTODATE = 0x01;
        /// 数据已修改（脏）
        const DIRTY    = 0x02;
        /// 需要刷新到磁盘
        const FLUSH    = 0x04;
        /// 临时块（不缓存）
        const TMP      = 0x08;
    }
}

/// 写入完成回调类型
///
/// 对应 lwext4 的 `end_write` 函数指针
pub type EndWriteCallback = Box<dyn FnOnce(Result<()>) + Send>;

/// 缓存块
///
/// 对应 lwext4 的 `struct ext4_buf`
///
/// # 重构简化
///
/// 使用 lru crate 后，CacheBuffer 不再需要维护复杂的引用计数和LRU状态：
/// - ✅ 删除 refctr：lru crate 自动管理块的生命周期
/// - ✅ 删除 lru_id：lru crate 内部维护访问顺序
/// - ✅ 删除 id：直接使用 lba 作为缓存key
///
/// 这使得结构更简单、更安全，不会出现引用计数泄漏或LRU索引不一致的问题。
pub struct CacheBuffer {
    /// 逻辑块地址
    pub lba: u64,

    /// 块数据
    pub data: Vec<u8>,

    /// 块状态标志
    flags: CacheFlags,

    /// 异步写入完成回调
    pub end_write: Option<EndWriteCallback>,
}

impl core::fmt::Debug for CacheBuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CacheBuffer")
            .field("lba", &self.lba)
            .field("data_len", &self.data.len())
            .field("flags", &self.flags)
            .field("end_write", &self.end_write.as_ref().map(|_| "<callback>"))
            .finish()
    }
}

impl CacheBuffer {
    /// 创建新的缓存块
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    /// * `block_size` - 块大小（字节）
    pub fn new(lba: u64, block_size: usize) -> Self {
        Self {
            lba,
            data: alloc::vec![0u8; block_size],
            flags: CacheFlags::empty(),
            end_write: None,
        }
    }

    /// 标记为脏（已修改）
    pub fn mark_dirty(&mut self) {
        self.flags.insert(CacheFlags::DIRTY);
    }

    /// 标记为干净（已写入磁盘）
    pub fn clear_dirty(&mut self) {
        self.flags.remove(CacheFlags::DIRTY);
    }

    /// 检查是否是脏块
    pub fn is_dirty(&self) -> bool {
        self.flags.contains(CacheFlags::DIRTY)
    }

    /// 标记数据有效
    pub fn mark_uptodate(&mut self) {
        self.flags.insert(CacheFlags::UPTODATE);
    }

    /// 检查数据是否有效
    pub fn is_uptodate(&self) -> bool {
        self.flags.contains(CacheFlags::UPTODATE)
    }

    /// 标记需要刷新
    pub fn mark_flush(&mut self) {
        self.flags.insert(CacheFlags::FLUSH);
    }

    /// 检查是否需要刷新
    pub fn needs_flush(&self) -> bool {
        self.flags.contains(CacheFlags::FLUSH)
    }

    /// 标记为临时块
    pub fn mark_tmp(&mut self) {
        self.flags.insert(CacheFlags::TMP);
    }

    /// 检查是否是临时块
    pub fn is_tmp(&self) -> bool {
        self.flags.contains(CacheFlags::TMP)
    }

    /// 设置写入完成回调
    pub fn set_end_write_callback(&mut self, callback: EndWriteCallback) {
        self.end_write = Some(callback);
    }

    /// 调用写入完成回调
    ///
    /// 如果设置了回调，会消费它并调用，传入写入结果
    pub fn invoke_end_write(&mut self, result: Result<()>) {
        if let Some(callback) = self.end_write.take() {
            callback(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buf = CacheBuffer::new(100, 4096);
        assert_eq!(buf.lba, 100);
        assert_eq!(buf.data.len(), 4096);
        assert_eq!(buf.flags, CacheFlags::empty());
    }

    #[test]
    fn test_dirty_flag() {
        let mut buf = CacheBuffer::new(100, 4096);

        assert!(!buf.is_dirty());

        buf.mark_dirty();
        assert!(buf.is_dirty());
        assert!(buf.flags.contains(CacheFlags::DIRTY));

        buf.clear_dirty();
        assert!(!buf.is_dirty());
    }

    #[test]
    fn test_uptodate_flag() {
        let mut buf = CacheBuffer::new(100, 4096);

        assert!(!buf.is_uptodate());

        buf.mark_uptodate();
        assert!(buf.is_uptodate());
        assert!(buf.flags.contains(CacheFlags::UPTODATE));
    }

    #[test]
    fn test_flush_flag() {
        let mut buf = CacheBuffer::new(100, 4096);

        assert!(!buf.needs_flush());

        buf.mark_flush();
        assert!(buf.needs_flush());
        assert!(buf.flags.contains(CacheFlags::FLUSH));
    }

    #[test]
    fn test_tmp_flag() {
        let mut buf = CacheBuffer::new(100, 4096);

        assert!(!buf.is_tmp());

        buf.mark_tmp();
        assert!(buf.is_tmp());
        assert!(buf.flags.contains(CacheFlags::TMP));
    }

    #[test]
    fn test_multiple_flags() {
        let mut buf = CacheBuffer::new(100, 4096);

        buf.mark_dirty();
        buf.mark_uptodate();
        buf.mark_flush();

        assert!(buf.is_dirty());
        assert!(buf.is_uptodate());
        assert!(buf.needs_flush());
        assert!(!buf.is_tmp());
    }

    #[test]
    fn test_end_write_callback() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicBool, Ordering};

        let mut buf = CacheBuffer::new(100, 4096);
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        buf.set_end_write_callback(Box::new(move |_result| {
            called_clone.store(true, Ordering::SeqCst);
        }));

        assert!(!called.load(Ordering::SeqCst));

        buf.invoke_end_write(Ok(()));
        assert!(called.load(Ordering::SeqCst));

        // 再次调用不应该有效果（回调已被消费）
        buf.invoke_end_write(Ok(()));
    }
}
