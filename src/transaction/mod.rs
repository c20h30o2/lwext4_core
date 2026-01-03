//! Transaction 系统
//!
//! 提供文件系统写操作的事务支持，确保操作的原子性。
//!
//! ## 模块结构
//!
//! - `simple` - 简化的事务系统（不使用 journal）
//!
//! ## 使用说明
//!
//! ### 简化 Transaction（开发/测试用）
//!
//! ⚠️ **警告**：`SimpleTransaction` 不提供崩溃恢复保证！
//! 只适用于开发、测试和可接受数据丢失风险的场景。
//!
//! ```rust,ignore
//! use lwext4_core::transaction::SimpleTransaction;
//!
//! // 开始事务
//! let mut trans = SimpleTransaction::begin(&mut bdev)?;
//!
//! // 执行修改
//! let mut block = trans.get_block(lba)?;
//! block.with_data_mut(|data| {
//!     // 修改块数据
//! })?;
//! trans.mark_dirty(lba)?;
//!
//! // 提交事务
//! trans.commit()?;
//! ```
//!
//! ### 完整 Journal Transaction（生产环境，未实现）
//!
//! ```rust,ignore
//! // 未来的 API 设计：
//! let mut trans = JournalTransaction::begin(fs)?;
//! // ... 操作 ...
//! trans.commit()?;  // 提供崩溃一致性保证
//! ```

mod simple;
mod journal;

pub use simple::SimpleTransaction;
pub use journal::{JournalTransaction, journal_init, journal_recover, journal_stop};

// Journal 功能说明：
// - JournalTransaction 当前为占位实现，所有操作都返回 Unsupported
// - 生产环境请继续使用 SimpleTransaction（无崩溃恢复）
// - 完整 Journal 实现将在后续版本提供
