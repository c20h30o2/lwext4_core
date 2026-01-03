//! 缓存块结构
//!
//! 对应 lwext4 的 `ext4_buf` 结构

use crate::error::Result;
use alloc::boxed::Box;
use alloc::vec::Vec;
use bitflags::bitflags;

/// 缓存块 ID，用于索引和关联
pub type BufferId = usize;

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
/// 在 lwext4 的 C 实现中，`ext4_buf` 使用嵌入式指针（RB_ENTRY、SLIST_ENTRY）
/// 来实现红黑树和链表成员关系。在 Rust 实现中，我们使用 `BufferId` 索引，
/// 通过外部的 BTreeMap 和 VecDeque 来管理这些关系，这样更加安全且符合 Rust 习惯。
///
/// # 字段说明
///
/// - `lba`: 逻辑块地址（Logical Block Address）
/// - `data`: 块数据缓冲区
/// - `refctr`: 引用计数，当 > 0 时块不能被驱逐
/// - `lru_id`: LRU 计数器值，用于 LRU 驱逐策略
/// - `flags`: 块状态标志
/// - `id`: 块的唯一标识符（Rust 特有，替代 C 中的指针）
/// - `end_write`: 异步写入完成回调
pub struct CacheBuffer {
    /// 逻辑块地址
    pub lba: u64,

    /// 块数据
    pub data: Vec<u8>,

    /// 引用计数
    pub refctr: u32,

    /// LRU 计数器值（越小越旧）
    pub lru_id: u32,

    /// 块状态标志
    pub flags: CacheFlags,

    /// 块 ID（用于索引）
    pub id: BufferId,

    /// 异步写入完成回调
    pub end_write: Option<EndWriteCallback>,
}

impl core::fmt::Debug for CacheBuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CacheBuffer")
            .field("lba", &self.lba)
            .field("data_len", &self.data.len())
            .field("refctr", &self.refctr)
            .field("lru_id", &self.lru_id)
            .field("flags", &self.flags)
            .field("id", &self.id)
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
    /// * `id` - 块 ID
    pub fn new(lba: u64, block_size: usize, id: BufferId) -> Self {
        Self {
            lba,
            data: alloc::vec![0u8; block_size],
            refctr: 0,
            lru_id: 0,
            flags: CacheFlags::empty(),
            id,
            end_write: None,
        }
    }

    /// 增加引用计数
    pub fn get(&mut self) {
        self.refctr = self.refctr.saturating_add(1);
    }

    /// 减少引用计数
    pub fn put(&mut self) {
        self.refctr = self.refctr.saturating_sub(1);
    }

    /// 检查是否正在被引用
    pub fn is_referenced(&self) -> bool {
        self.refctr > 0
    }

    /// 标记为脏（已修改）
    pub fn mark_dirty(&mut self) {
        self.flags.insert(CacheFlags::DIRTY);
    }

    /// 标记为干净（已写入磁盘）
    pub fn mark_clean(&mut self) {
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
        let buf = CacheBuffer::new(100, 4096, 0);
        assert_eq!(buf.lba, 100);
        assert_eq!(buf.data.len(), 4096);
        assert_eq!(buf.refctr, 0);
        assert_eq!(buf.lru_id, 0);
        assert_eq!(buf.flags, CacheFlags::empty());
        assert!(!buf.is_referenced());
    }

    #[test]
    fn test_reference_counting() {
        let mut buf = CacheBuffer::new(100, 4096, 0);

        assert!(!buf.is_referenced());

        buf.get();
        assert_eq!(buf.refctr, 1);
        assert!(buf.is_referenced());

        buf.get();
        assert_eq!(buf.refctr, 2);

        buf.put();
        assert_eq!(buf.refctr, 1);
        assert!(buf.is_referenced());

        buf.put();
        assert_eq!(buf.refctr, 0);
        assert!(!buf.is_referenced());

        // 测试饱和减法
        buf.put();
        assert_eq!(buf.refctr, 0);
    }

    #[test]
    fn test_dirty_flag() {
        let mut buf = CacheBuffer::new(100, 4096, 0);

        assert!(!buf.is_dirty());

        buf.mark_dirty();
        assert!(buf.is_dirty());
        assert!(buf.flags.contains(CacheFlags::DIRTY));

        buf.mark_clean();
        assert!(!buf.is_dirty());
    }

    #[test]
    fn test_uptodate_flag() {
        let mut buf = CacheBuffer::new(100, 4096, 0);

        assert!(!buf.is_uptodate());

        buf.mark_uptodate();
        assert!(buf.is_uptodate());
        assert!(buf.flags.contains(CacheFlags::UPTODATE));
    }

    #[test]
    fn test_flush_flag() {
        let mut buf = CacheBuffer::new(100, 4096, 0);

        assert!(!buf.needs_flush());

        buf.mark_flush();
        assert!(buf.needs_flush());
        assert!(buf.flags.contains(CacheFlags::FLUSH));
    }

    #[test]
    fn test_tmp_flag() {
        let mut buf = CacheBuffer::new(100, 4096, 0);

        assert!(!buf.is_tmp());

        buf.mark_tmp();
        assert!(buf.is_tmp());
        assert!(buf.flags.contains(CacheFlags::TMP));
    }

    #[test]
    fn test_multiple_flags() {
        let mut buf = CacheBuffer::new(100, 4096, 0);

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

        let mut buf = CacheBuffer::new(100, 4096, 0);
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
