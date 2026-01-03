//! 块缓存实现
//!
//! 对应 lwext4 的 `ext4_bcache` 结构和相关函数
//!
//! 这个模块实现了一个完整的块缓存系统，使用双索引（LBA + LRU）和脏块列表。
//! 相比 lwext4 的 C 实现（使用嵌入式红黑树），这里使用 Rust 的 `BTreeMap` 和 `VecDeque`，
//! 提供了更好的类型安全性和内存安全性。
//! 
//! cache模块本身不与读磁盘的逻辑交互， 只为写磁盘提供flush接口， cache作为工具为device服务 
//! 注意： block_cache本身管理cachebuffer并不处理脏块写回磁盘的任务， 只提供flush接口， cachebuffer只在两种情况下被彻底从buffer数组中移除：
//! 1. 调用evict_one 
//! 2. 调用drop_buffer 
//! 这意味着无法再找到对应的lba的cachebuffer, 该lba原来占有的buffer数组槽位已经被标记为None， 只能再次为这个块alloc一个槽位
//! 并且如上文所说， 这两个函数都不处理脏块写回， 只处理对应的索引的删除任务

use crate::{
    block::BlockDevice,
    error::{Error, ErrorKind, Result},
};

use super::buffer::{BufferId, CacheBuffer};
use alloc::{collections::BTreeMap, collections::VecDeque, vec::Vec};

/// 默认缓存块数量
///
/// 对应 lwext4 的 `CONFIG_BLOCK_DEV_CACHE_SIZE`
pub const DEFAULT_CACHE_SIZE: usize = 8;

/// 块缓存
///
/// 对应 lwext4 的 `struct ext4_bcache`
///
/// # 实现原理
///
/// 块缓存使用三个关键数据结构来管理缓存块：
///
/// 1. **LBA 索引** (`lba_index`): 通过逻辑块地址快速查找块
///    - C 实现: `RB_HEAD(ext4_buf_lba, ext4_buf) lba_root`
///    - Rust 实现: `BTreeMap<u64, BufferId>`
///
/// 2. **LRU 索引** (`lru_index`): 通过 LRU 计数器查找最久未使用的块
///    - C 实现: `RB_HEAD(ext4_buf_lru, ext4_buf) lru_root`
///    - Rust 实现: `BTreeMap<u32, BufferId>`
///    - 注意: 引用计数 > 0 的块不在此索引中
///
/// 3. **脏块列表** (`dirty_list`): 追踪需要写回磁盘的修改块
///    - C 实现: `SLIST_HEAD(ext4_buf_dirty, ext4_buf) dirty_list`
///    - Rust 实现: `VecDeque<BufferId>`
///
/// # LRU 驱逐策略
///
/// 当缓存满时，系统会驱逐最久未使用的块。LRU 计数器递增，新访问的块获得新的
/// LRU ID。引用计数 > 0 的块被视为"固定"，不能被驱逐。
///
/// # 示例
///
/// ```rust,ignore
/// use lwext4_core::cache::BlockCache;
///
/// let mut cache = BlockCache::new(8, 4096);
///
/// // 分配块
/// let (buf, is_new) = cache.alloc(100)?;
/// buf.mark_dirty();
///
/// // 查找块
/// if let Some(buf) = cache.find_get(100) {
///     // 使用块...
/// }
///
/// // 释放块
/// cache.free(100)?;
///
/// // 刷新所有脏块
/// cache.flush_all(&mut block_device)?;
/// ```
pub struct BlockCache {
    /// 缓存容量（最大块数）
    capacity: usize,

    /// 块大小（字节）
    block_size: usize,

    /// LBA 索引：逻辑块地址 -> 块 ID
    ///
    /// 对应 lwext4 的 `lba_root` 红黑树
    lba_index: BTreeMap<u64, BufferId>,

    /// LRU 索引：LRU 计数器 -> 块 ID
    ///
    /// 对应 lwext4 的 `lru_root` 红黑树
    ///
    /// **重要**: 只有 refctr == 0 的块才在此索引中！
    /// 当块的引用计数 > 0 时，它会被从 LRU 索引中移除，
    /// 因为正在使用的块不应该被驱逐。
    lru_index: BTreeMap<u32, BufferId>,

    /// 块存储：块 ID -> 块数据
    ///
    /// 使用 `Option` 以支持槽位重用（当块被释放时设为 None）
    buffers: Vec<Option<CacheBuffer>>,

    /// 脏块列表（需要写回磁盘）
    ///
    /// 对应 lwext4 的 `dirty_list` 单链表
    dirty_list: VecDeque<BufferId>,

    /// 空闲槽位列表
    ///
    /// 追踪 `buffers` 中的空闲位置，用于块分配
    free_list: Vec<BufferId>,

    /// LRU 计数器（递增）
    ///
    /// 对应 lwext4 的 `lru_ctr`
    lru_counter: u32,

    /// 当前引用的块数量
    ///
    /// 对应 lwext4 的 `ref_blocks`
    ref_blocks: u32,

    /// 最大引用块数量限制（None 表示无限制）
    ///
    /// 对应 lwext4 的 `max_ctr`
    max_ref_blocks: Option<u32>,

    /// 禁用驱逐标志
    ///
    /// 当为 true 时，不会驱逐任何块（即使缓存满）
    ///
    /// 对应 lwext4 的 `dont_shake`
    dont_shake: bool,

    /// 写回模式引用计数
    ///
    /// 对应 lwext4 的 `cache_write_back`
    ///
    /// 当 > 0 时启用写回模式（延迟写入）
    /// 当 == 0 时启用写穿模式（立即写入）
    write_back_counter: u32,
}

impl BlockCache {
    /// 创建新的块缓存
    ///
    /// # 参数
    ///
    /// * `capacity` - 缓存容量（块数量）
    /// * `block_size` - 块大小（字节）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let cache = BlockCache::new(8, 4096);
    /// ```
    pub fn new(capacity: usize, block_size: usize) -> Self {
        Self {
            capacity,
            block_size,
            lba_index: BTreeMap::new(),
            lru_index: BTreeMap::new(),
            buffers: Vec::with_capacity(capacity),
            dirty_list: VecDeque::new(),
            free_list: Vec::new(),
            lru_counter: 0,
            ref_blocks: 0,
            max_ref_blocks: None,
            dont_shake: false,
            write_back_counter: 0,
        }
    }

    /// 设置最大引用块数量限制
    ///
    /// # 参数
    ///
    /// * `max` - 最大引用块数量（None 表示无限制）
    pub fn set_max_ref_blocks(&mut self, max: Option<u32>) {
        self.max_ref_blocks = max;
    }

    /// 禁用/启用驱逐
    ///
    /// 当禁用驱逐时，缓存满时会返回错误而不是驱逐旧块
    ///
    /// # 参数
    ///
    /// * `dont_shake` - true 禁用驱逐，false 启用驱逐
    pub fn set_dont_shake(&mut self, dont_shake: bool) {
        self.dont_shake = dont_shake;
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            capacity: self.capacity,
            used: self.lba_index.len(),
            ref_blocks: self.ref_blocks as usize,
            dirty_blocks: self.dirty_list.len(),
            lru_counter: self.lru_counter,
        }
    }

    /// 查找块（不增加引用计数）
    ///
    /// 对应 lwext4 的 `ext4_bcache_find` 功能（虽然 lwext4 没有独立的 find 函数）
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 返回
    ///
    /// 找到则返回块的引用，否则返回 None
    pub fn lookup(&self, lba: u64) -> Option<&CacheBuffer> {
        let id = *self.lba_index.get(&lba)?;
        self.buffers.get(id)?.as_ref()
    }

    /// 查找块并增加引用计数
    ///
    /// 对应 lwext4 的 `ext4_bcache_find_get` 函数
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 返回
    ///
    /// 找到则返回块的可变引用，否则返回 None
    ///
    /// # 副作用
    ///
    /// - 增加块的引用计数
    /// - 如果块之前引用计数为 0，会从 LRU 索引中移除
    /// - 增加全局引用块计数
    pub fn find_get(&mut self, lba: u64) -> Option<&mut CacheBuffer> {
        let id = *self.lba_index.get(&lba)?;
        let buf = self.buffers.get_mut(id)?.as_mut()?;

        // 如果之前未被引用，从 LRU 索引中移除
        if !buf.is_referenced() {
            self.lru_index.remove(&buf.lru_id);
        }

        buf.get();
        self.ref_blocks += 1;

        Some(buf)
    }

    /// 查找 LRU 值最低的块（最久未使用）
    ///
    /// 对应 lwext4 的 `ext4_buf_lowest_lru` 函数
    ///
    /// # 返回
    ///
    /// 返回最久未使用的块（引用计数为 0），如果所有块都被引用则返回 None
    ///
    /// # 说明
    ///
    /// 只有引用计数为 0 的块才会在 LRU 索引中，所以这个函数实际上是
    /// 查找 lru_index 中键值最小的条目。
    pub fn lowest_lru(&self) -> Option<&CacheBuffer> {
        // BTreeMap 是有序的，first_key_value() 返回最小键
        let (_lru_id, &buf_id) = self.lru_index.first_key_value()?;
        self.buffers.get(buf_id)?.as_ref()
    }

    /// 驱逐一个块
    ///
    /// 对应 lwext4 的驱逐逻辑（在 `ext4_bcache_alloc` 中）
    ///
    /// # 返回
    ///
    /// 成功返回被驱逐块的 ID，如果无法驱逐则返回错误
    ///
    /// # 错误
    ///
    /// - 如果 `dont_shake` 为 true，返回错误
    /// - 如果所有块都被引用，返回错误
    fn evict_one(&mut self) -> Result<BufferId> {
        if self.dont_shake {
            return Err(Error::new(
                ErrorKind::NoSpace,
                "Cache full and eviction disabled",
            ));
        }

        // 找到 LRU 最低的块
        let lru_buf = self.lowest_lru().ok_or_else(|| {
            Error::new(
                ErrorKind::NoSpace,
                "All cache blocks are referenced, cannot evict",
            )
        })?;

        let lba = lru_buf.lba;
        let id = lru_buf.id;
        let lru_id = lru_buf.lru_id;

        // 从所有索引中移除
        self.lba_index.remove(&lba);
        self.lru_index.remove(&lru_id);

        // 从脏列表中移除（如果存在）
        if let Some(pos) = self.dirty_list.iter().position(|&x| x == id) {
            self.dirty_list.remove(pos);
        }

        // 清空块槽位
        self.buffers[id] = None;

        Ok(id)
    }

    /// 分配或复用块 ID
    ///
    /// # 返回
    ///
    /// 可用的块 ID
    fn allocate_slot(&mut self) -> Result<BufferId> {
        // 1. 先尝试使用空闲列表
        if let Some(id) = self.free_list.pop() {
            return Ok(id);
        }

        // 2. 如果缓存未满，分配新槽位
        if self.buffers.len() < self.capacity {
            let id = self.buffers.len();
            self.buffers.push(None);
            return Ok(id);
        }

        // 3. 缓存已满，需要驱逐
        self.evict_one()
    }

    /// 分配缓存块
    ///
    /// 对应 lwext4 的 `ext4_bcache_alloc` 函数
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 返回
    ///
    /// 成功返回 `(块的可变引用, 是否是新分配)`
    ///
    /// - 如果块已存在于缓存中，返回 `(块, false)` 并增加引用计数
    /// - 如果块不存在，分配新块返回 `(块, true)` 并设置引用计数为 1
    ///
    /// # 错误
    ///
    /// - 如果达到最大引用块数限制，返回错误
    /// - 如果缓存满且无法驱逐，返回错误
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (buf, is_new) = cache.alloc(100)?;
    /// if is_new {
    ///     // 读取数据到 buf.data
    ///     block_device.read_block(100, &mut buf.data)?;
    ///     buf.mark_uptodate();
    /// }
    /// ```
    pub fn alloc(&mut self, lba: u64) -> Result<(&mut CacheBuffer, bool)> {
        // 先检查块是否已存在（只读检查）
        let exists = self.lba_index.contains_key(&lba);

        if exists {
            // 块已存在，增加引用计数并返回
            let buf = self.find_get(lba).unwrap(); // 我们知道它存在
            return Ok((buf, false));
        }

        // 块不存在，需要分配新块

        // 检查是否达到最大引用块数限制
        if let Some(max) = self.max_ref_blocks {
            if self.ref_blocks >= max {
                return Err(Error::new(
                    ErrorKind::NoSpace,
                    "Maximum referenced blocks limit reached",
                ));
            }
        }

        // 分配槽位
        let id = self.allocate_slot()?;

        // 创建新缓存块
        let lru_id = self.next_lru_id();
        let mut buf = CacheBuffer::new(lba, self.block_size, id);
        buf.lru_id = lru_id;
        buf.get(); // 引用计数设为 1

        // 插入索引
        self.lba_index.insert(lba, id);
        // 注意：新分配的块引用计数为 1，所以不放入 LRU 索引

        // 存储块
        self.buffers[id] = Some(buf);
        self.ref_blocks += 1;

        // 返回块的可变引用
        let buf = self.buffers[id].as_mut().unwrap();
        Ok((buf, true))
    }

    /// 释放块（减少引用计数）
    ///
    /// 对应 lwext4 的 `ext4_bcache_free` 函数
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 返回
    ///
    /// 成功返回块当前的引用计数
    ///
    /// # 错误
    ///
    /// 如果块不存在，返回错误
    ///
    /// # 副作用
    ///
    /// - 减少块的引用计数
    /// - 如果引用计数降为 0，将块加入 LRU 索引
    /// - 减少全局引用块计数
    pub fn free(&mut self, lba: u64) -> Result<u32> {
        let id = *self
            .lba_index
            .get(&lba)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block not in cache"))?;

        let buf = self.buffers[id]
            .as_mut()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Invalid buffer slot"))?;

        if !buf.is_referenced() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Block reference count already zero",
            ));
        }

        buf.put();
        self.ref_blocks = self.ref_blocks.saturating_sub(1);

        // 如果引用计数降为 0，加入 LRU 索引
        // ⚠️ 但是 dirty 块不应该被加入 LRU！dirty 块即使 refctr = 0 也不应该被驱逐
        // 它们应该保持在缓存中直到写回磁盘
        if !buf.is_referenced() && !buf.is_dirty() {
            let lru_id = buf.lru_id;
            let buf_id = buf.id;
            self.lru_index.insert(lru_id, buf_id);
        }

        Ok(buf.refctr)
    }

    /// 丢弃块（从缓存中完全移除）
    ///
    /// 对应 lwext4 中的驱逐逻辑
    ///
    /// # 参数
    ///
    /// * `id` - 块 ID
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    ///
    /// # 副作用
    ///
    /// - 从所有索引中移除块
    /// - 将槽位加入空闲列表
    ///
    /// # 注意
    ///
    /// 调用者需要确保块的引用计数为 0
    pub fn drop_buffer(&mut self, id: BufferId) -> Result<()> {
        let buf = self.buffers[id]
            .take()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Buffer slot empty"))?;

        if buf.is_referenced() {
            // 恢复块（不应该丢弃被引用的块）
            self.buffers[id] = Some(buf);
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Cannot drop referenced buffer",
            ));
        }

        // 从索引中移除
        self.lba_index.remove(&buf.lba);
        self.lru_index.remove(&buf.lru_id);

        // 从脏列表中移除
        if let Some(pos) = self.dirty_list.iter().position(|&x| x == id) {
            self.dirty_list.remove(pos);
        }

        // 加入空闲列表
        self.free_list.push(id);

        Ok(())
    }

    /// 使块失效（从缓存中移除）
    ///
    /// 对应 lwext4 的 `ext4_bcache_invalidate_lba` 函数
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())，如果块不存在也返回 Ok(())
    ///
    /// # 错误
    ///
    /// 如果块正在被引用，返回错误
    pub fn invalidate_buffer(&mut self, lba: u64) -> Result<()> {
        if let Some(&id) = self.lba_index.get(&lba) {
            self.drop_buffer(id)?;
        }
        Ok(())
    }

    /// 使一组连续块失效
    ///
    /// 对应 lwext4 的部分实现（lwext4 没有直接的范围失效函数，但有类似逻辑）
    ///
    /// # 参数
    ///
    /// * `from` - 起始逻辑块地址
    /// * `count` - 块数量
    ///
    /// # 返回
    ///
    /// 成功返回失效的块数量
    ///
    /// # 错误
    ///
    /// 如果任何块正在被引用，返回错误
    pub fn invalidate_range(&mut self, from: u64, count: u32) -> Result<usize> {
        let mut invalidated = 0;

        for offset in 0..count {
            let lba = from + offset as u64;
            if self.invalidate_buffer(lba).is_ok() {
                invalidated += 1;
            }
        }

        Ok(invalidated)
    }

    /// 将块标记为脏并加入脏列表
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 错误
    ///
    /// 如果块不存在，返回错误
    pub fn mark_dirty(&mut self, lba: u64) -> Result<()> {
        let id = *self
            .lba_index
            .get(&lba)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block not in cache"))?;

        let buf = self.buffers[id]
            .as_mut()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Invalid buffer slot"))?;

        if !buf.is_dirty() {
            buf.mark_dirty();
            self.dirty_list.push_back(id);
        }

        Ok(())
    }

    /// 刷新所有脏块到磁盘
    ///
    /// 对应 lwext4 的 `ext4_block_cache_flush` 函数
    ///
    /// # 参数
    ///
    /// * `device` - 块设备（直接操作 BlockDevice trait）
    /// * `sector_size` - 扇区大小
    /// * `partition_offset` - 分区偏移（字节）
    ///
    /// # 返回
    ///
    /// 成功返回刷新的块数量
    ///
    /// # 错误
    ///
    /// 如果任何写入操作失败，返回错误
    pub fn flush_all<D: BlockDevice>(
        &mut self,
        device: &mut D,
        sector_size: u32,
        partition_offset: u64,
    ) -> Result<usize> {
        let mut flushed = 0;

        // 处理所有脏块
        while let Some(id) = self.dirty_list.pop_front() {
            if let Some(buf) = &mut self.buffers[id] {
                if buf.is_dirty() {
                    // 计算物理扇区地址
                    let byte_offset = buf.lba * self.block_size as u64 + partition_offset;
                    let pba = byte_offset / sector_size as u64;
                    let count = self.block_size as u32 / sector_size;

                    // 写入块到设备
                    let result = device.write_blocks(pba, count, &buf.data);

                    // 检查写入结果
                    let is_ok = result.is_ok();

                    // 调用写入完成回调
                    buf.invoke_end_write(result.map(|_| ()));

                    // 如果写入成功，标记为干净
                    if is_ok {
                        buf.mark_clean();
                        flushed += 1;
                        // 如果块的引用计数为 0，现在可以加入 LRU 索引了
                        // （之前 dirty 时不能加入 LRU）
                        if !buf.is_referenced() {
                            let lru_id = buf.lru_id;
                            self.lru_index.insert(lru_id, id);
                        }
                    } else {
                        // 写入失败，重新加入脏列表
                        self.dirty_list.push_back(id);
                        return Err(Error::new(ErrorKind::Io, "Failed to write block"));
                    }
                }
            }
        }

        Ok(flushed)
    }

    /// 刷新指定逻辑块地址的缓存到磁盘
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    /// * `device` - 块设备（直接操作 BlockDevice trait）
    /// * `sector_size` - 扇区大小
    /// * `partition_offset` - 分区偏移（字节）
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    ///
    /// # 错误
    ///
    /// 如果块不在缓存中或写入失败，返回错误
    pub fn flush_lba<D: BlockDevice>(
        &mut self,
        lba: u64,
        device: &mut D,
        sector_size: u32,
        partition_offset: u64,
    ) -> Result<()> {
        let id = *self
            .lba_index
            .get(&lba)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block not in cache"))?;

        if self.dirty_list.contains(&id) {
            let buf = self.buffers[id].as_mut().unwrap();
            if buf.is_dirty() {
                // 计算物理扇区地址
                let byte_offset = buf.lba * self.block_size as u64 + partition_offset;
                let pba = byte_offset / sector_size as u64;
                let count = self.block_size as u32 / sector_size;

                // 写入块到设备
                let result = device.write_blocks(pba, count, &buf.data);
                let is_ok = result.is_ok();

                // 调用写入完成回调
                buf.invoke_end_write(result.map(|_| ()));

                if is_ok {
                    buf.mark_clean();
                    // 从脏列表中移除
                    if let Some(index) = self.dirty_list.iter().position(|x| x == &id) {
                        self.dirty_list.remove(index);
                    }
                    // 如果块的引用计数为 0，现在可以加入 LRU 索引了
                    if !buf.is_referenced() {
                        let lru_id = buf.lru_id;
                        self.lru_index.insert(lru_id, id);
                    }
                } else {
                    return Err(Error::new(ErrorKind::Io, "Failed to write block"));
                }
            }
        }

        Ok(())
    }

    /// 从缓存读取块数据
    ///
    /// 如果块在缓存中且数据有效，返回块数据的引用。
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 返回
    ///
    /// 成功返回块数据的切片引用
    ///
    /// # 错误
    ///
    /// 如果块不在缓存中或数据无效，返回错误
    pub fn read_block(&self, lba: u64) -> Result<&[u8]> {
        let id = *self
            .lba_index
            .get(&lba)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block not in cache"))?;

        let buf = self.buffers[id]
            .as_ref()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Invalid buffer slot"))?;

        if !buf.is_uptodate() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Block data not valid",
            ));
        }

        Ok(&buf.data)
    }

    /// 将数据写入缓存块
    ///
    /// 将数据写入缓存块并标记为脏。如果块不在缓存中，返回错误。
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    /// * `data` - 要写入的数据
    ///
    /// # 返回
    ///
    /// 成功返回写入的字节数
    ///
    /// # 错误
    ///
    /// 如果块不在缓存中，返回错误
    pub fn write_block(&mut self, lba: u64, data: &[u8]) -> Result<usize> {
        let id = *self
            .lba_index
            .get(&lba)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block not in cache"))?;

        let buf = self.buffers[id]
            .as_mut()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Invalid buffer slot"))?;

        if data.len() > buf.data.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Data too large for block",
            ));
        }

        // 写入数据
        buf.data[..data.len()].copy_from_slice(data);

        // 标记为脏和有效
        buf.mark_uptodate();
        if !buf.is_dirty() {
            buf.mark_dirty();
            self.dirty_list.push_back(id);
        }

        Ok(data.len())
    }

    /// 生成下一个 LRU ID
    fn next_lru_id(&mut self) -> u32 {
        let id = self.lru_counter;
        self.lru_counter = self.lru_counter.wrapping_add(1);
        id
    }

    /// 启用写回模式
    ///
    /// 对应 lwext4 的 `ext4_block_cache_write_back(bdev, 1)`
    ///
    /// 启用后，脏块会保留在缓存中，直到显式刷新或驱逐。
    /// 可以多次调用以实现嵌套的写回模式控制。
    pub fn enable_write_back(&mut self) {
        self.write_back_counter = self.write_back_counter.saturating_add(1);
    }

    /// 禁用写回模式
    ///
    /// 对应 lwext4 的 `ext4_block_cache_write_back(bdev, 0)`
    ///
    /// 如果引用计数降为 0，会立即刷新所有脏块到设备。
    ///
    /// # 参数
    ///
    /// * `device` - 块设备
    /// * `sector_size` - 扇区大小
    /// * `partition_offset` - 分区偏移
    ///
    /// # 返回
    ///
    /// 成功返回刷新的块数量，如果仍处于写回模式则返回 0
    pub fn disable_write_back<D: BlockDevice>(
        &mut self,
        device: &mut D,
        sector_size: u32,
        partition_offset: u64,
    ) -> Result<usize> {
        if self.write_back_counter > 0 {
            self.write_back_counter -= 1;
        }

        // 如果计数器降为 0，刷新所有脏块
        if self.write_back_counter == 0 {
            return self.flush_all(device, sector_size, partition_offset);
        }

        Ok(0)
    }

    /// 检查是否启用写回模式
    pub fn is_write_back_enabled(&self) -> bool {
        self.write_back_counter > 0
    }

    /// 获取写回模式引用计数
    pub fn write_back_counter(&self) -> u32 {
        self.write_back_counter
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    /// 缓存容量
    pub capacity: usize,
    /// 已使用的块数
    pub used: usize,
    /// 被引用的块数
    pub ref_blocks: usize,
    /// 脏块数量
    pub dirty_blocks: usize,
    /// 当前 LRU 计数器值
    pub lru_counter: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = BlockCache::new(8, 4096);
        let stats = cache.stats();
        assert_eq!(stats.capacity, 8);
        assert_eq!(stats.used, 0);
        assert_eq!(stats.ref_blocks, 0);
        assert_eq!(stats.dirty_blocks, 0);
    }

    #[test]
    fn test_alloc_new_block() {
        let mut cache = BlockCache::new(8, 4096);

        let (buf, is_new) = cache.alloc(100).unwrap();
        assert!(is_new);
        assert_eq!(buf.lba, 100);
        assert_eq!(buf.refctr, 1);
        assert_eq!(buf.data.len(), 4096);

        let stats = cache.stats();
        assert_eq!(stats.used, 1);
        assert_eq!(stats.ref_blocks, 1);
    }

    #[test]
    fn test_alloc_existing_block() {
        let mut cache = BlockCache::new(8, 4096);

        // 第一次分配
        let (buf1, is_new1) = cache.alloc(100).unwrap();
        assert!(is_new1);
        assert_eq!(buf1.refctr, 1);

        // 第二次分配相同块
        let (buf2, is_new2) = cache.alloc(100).unwrap();
        assert!(!is_new2);
        assert_eq!(buf2.refctr, 2); // 引用计数增加

        let stats = cache.stats();
        assert_eq!(stats.used, 1); // 仍然只有一个块
        assert_eq!(stats.ref_blocks, 2); // 但引用计数为 2
    }

    #[test]
    fn test_free_block() {
        let mut cache = BlockCache::new(8, 4096);

        cache.alloc(100).unwrap();
        assert_eq!(cache.stats().ref_blocks, 1);

        let refctr = cache.free(100).unwrap();
        assert_eq!(refctr, 0);
        assert_eq!(cache.stats().ref_blocks, 0);
    }

    #[test]
    fn test_find_get() {
        let mut cache = BlockCache::new(8, 4096);

        // 分配并释放块
        cache.alloc(100).unwrap();
        cache.free(100).unwrap();

        // 查找块
        let buf = cache.find_get(100).unwrap();
        assert_eq!(buf.lba, 100);
        assert_eq!(buf.refctr, 1);

        // 查找不存在的块
        assert!(cache.find_get(200).is_none());
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = BlockCache::new(2, 4096);

        // 填满缓存
        cache.alloc(100).unwrap();
        cache.alloc(200).unwrap();

        // 释放第一个块
        cache.free(100).unwrap();
        cache.free(200).unwrap();

        // 分配第三个块，应该驱逐 LRU 最低的块（100）
        cache.alloc(300).unwrap();

        // 100 应该被驱逐
        assert!(cache.lookup(100).is_none());
        assert!(cache.lookup(200).is_some());
        assert!(cache.lookup(300).is_some());
    }

    #[test]
    fn test_cannot_evict_referenced_block() {
        let mut cache = BlockCache::new(2, 4096);

        // 填满缓存，但不释放
        cache.alloc(100).unwrap();
        cache.alloc(200).unwrap();

        // 尝试分配第三个块，应该失败（所有块都被引用）
        let result = cache.alloc(300);
        assert!(result.is_err());
    }

    #[test]
    fn test_dirty_list() {
        let mut cache = BlockCache::new(8, 4096);

        cache.alloc(100).unwrap();
        cache.mark_dirty(100).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.dirty_blocks, 1);
    }

    #[test]
    fn test_invalidate_buffer() {
        let mut cache = BlockCache::new(8, 4096);

        cache.alloc(100).unwrap();
        cache.free(100).unwrap();

        // 使块失效
        cache.invalidate_buffer(100).unwrap();

        // 块应该不存在
        assert!(cache.lookup(100).is_none());
    }

    #[test]
    fn test_invalidate_range() {
        let mut cache = BlockCache::new(8, 4096);

        for i in 100..105 {
            cache.alloc(i).unwrap();
            cache.free(i).unwrap();
        }

        // 使范围失效
        let count = cache.invalidate_range(100, 5).unwrap();
        assert_eq!(count, 5);

        // 所有块都应该不存在
        for i in 100..105 {
            assert!(cache.lookup(i).is_none());
        }
    }

    #[test]
    fn test_dont_shake() {
        let mut cache = BlockCache::new(2, 4096);
        cache.set_dont_shake(true);

        // 填满缓存
        cache.alloc(100).unwrap();
        cache.alloc(200).unwrap();

        cache.free(100).unwrap();
        cache.free(200).unwrap();

        // 尝试分配第三个块，应该失败（驱逐被禁用）
        let result = cache.alloc(300);
        assert!(result.is_err());
    }

    #[test]
    fn test_max_ref_blocks() {
        let mut cache = BlockCache::new(8, 4096);
        cache.set_max_ref_blocks(Some(2));

        cache.alloc(100).unwrap();
        cache.alloc(200).unwrap();

        // 尝试分配第三个块，应该失败（达到引用限制）
        let result = cache.alloc(300);
        assert!(result.is_err());
    }
}
