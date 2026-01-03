//! 块缓存模块
//!
//! 这个模块提供了完整的块缓存实现，对应 lwext4 的 bcache 功能。
//!
//! # 主要组件
//!
//! - [`CacheBuffer`] - 单个缓存块，包含数据和元数据
//! - [`BlockCache`] - 块缓存管理器，使用双索引（LBA + LRU）
//! - [`CacheFlags`] - 缓存块状态标志
//! - [`CacheStats`] - 缓存统计信息
//!
//! # 设计原理
//!
//! 本模块是 lwext4 C 库中 `ext4_bcache.c` 的纯 Rust 重写。关键改进：
//!
//! 1. **数据结构**：使用 `BTreeMap` 替代嵌入式红黑树，更安全且性能相当
//! 2. **内存安全**：使用索引替代原始指针，避免悬空指针和内存泄漏
//! 3. **类型安全**：使用强类型和 `bitflags` 替代 C 的 int 标志
//! 4. **所有权**：Rust 的所有权系统确保缓存块不会被意外共享或修改
//!
//! # 与 lwext4 的对应关系
//!
//! | lwext4 C                          | lwext4-rust                       |
//! |-----------------------------------|-----------------------------------|
//! | `struct ext4_buf`                 | [`CacheBuffer`]                   |
//! | `struct ext4_bcache`              | [`BlockCache`]                    |
//! | `RB_HEAD(lba_root)`               | `BTreeMap<u64, BufferId>`         |
//! | `RB_HEAD(lru_root)`               | `BTreeMap<u32, BufferId>`         |
//! | `SLIST_HEAD(dirty_list)`          | `VecDeque<BufferId>`              |
//! | `ext4_bcache_alloc()`             | [`BlockCache::alloc()`]           |
//! | `ext4_bcache_free()`              | [`BlockCache::free()`]            |
//! | `ext4_bcache_find_get()`          | [`BlockCache::find_get()`]        |
//! | `ext4_buf_lowest_lru()`           | [`BlockCache::lowest_lru()`]      |
//! | `ext4_bcache_invalidate_lba()`    | [`BlockCache::invalidate_buffer()`]|
//! | `ext4_block_cache_flush()`        | [`BlockCache::flush_all()`]       |
//!
//! # 功能完整性
//!
//! ✅ LBA 索引（快速查找）
//! ✅ LRU 驱逐策略
//! ✅ 引用计数
//! ✅ 脏块跟踪
//! ✅ 范围失效
//! ✅ 异步写入回调
//! ✅ 驱逐控制（dont_shake）
//! ✅ 引用限制（max_ref_blocks）
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use lwext4_core::cache::{BlockCache, DEFAULT_CACHE_SIZE};
//!
//! // 创建缓存（8 个块，每块 4096 字节）
//! let mut cache = BlockCache::new(DEFAULT_CACHE_SIZE, 4096);
//!
//! // 分配块
//! let (buf, is_new) = cache.alloc(100)?;
//! if is_new {
//!     // 从设备读取数据
//!     block_device.read_block(100, &mut buf.data)?;
//!     buf.mark_uptodate();
//! }
//!
//! // 修改数据
//! buf.data[0] = 42;
//! buf.mark_dirty();
//! cache.mark_dirty(100)?;
//!
//! // 释放块（减少引用计数）
//! cache.free(100)?;
//!
//! // 刷新所有脏块
//! cache.flush_all(&mut block_device)?;
//!
//! // 查看统计信息
//! let stats = cache.stats();
//! println!("Cache: {}/{} used, {} dirty",
//!          stats.used, stats.capacity, stats.dirty_blocks);
//! ```
//!
//! # 性能特性
//!
//! - **查找**: O(log n) - BTreeMap 查找
//! - **插入**: O(log n) - BTreeMap 插入
//! - **LRU 查找**: O(log n) - BTreeMap 最小键查找
//! - **驱逐**: O(log n) - LRU 查找 + 删除
//! - **刷新**: O(k) - k 为脏块数量
//!
//! 相比 lwext4 的红黑树实现，BTreeMap 在实践中通常有更好的缓存局部性。
//!
//! # 内存分配要求
//!
//! 本模块依赖 `alloc` crate，需要用户提供全局分配器。
//! 参见 [`alloc::alloc::GlobalAlloc`] 和 `#[global_allocator]`。

mod buffer;
mod block_cache;

pub use buffer::{BufferId, CacheBuffer, CacheFlags, EndWriteCallback};
pub use block_cache::{BlockCache, CacheStats, DEFAULT_CACHE_SIZE};
