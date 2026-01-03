//! 简化的事务系统实现
//!
//! ⚠️ **重要警告**：此实现不提供崩溃恢复保证！
//!
//! ## 设计目标
//!
//! 1. 提供基本的事务API，与完整 journal 系统接口兼容
//! 2. 快速实现，支持写操作开发和测试
//! 3. 为后续 journal 集成留出接口
//!
//! ## 限制
//!
//! - ❌ 无崩溃恢复保证
//! - ❌ 无原子性保证（部分写入可能发生）
//! - ❌ 不适合生产环境
//! - ❌ 不适合多用户并发环境
//!
//! ## 适用场景
//!
//! - ✅ 开发和测试
//! - ✅ 单用户环境
//! - ✅ 可接受数据丢失风险的场景
//!
//! ## 工作原理
//!
//! 1. **记录修改**: 跟踪所有被修改的块号
//! 2. **延迟写入**: 修改在 block cache 中保持，不立即写入磁盘
//! 3. **提交**: 将所有脏块刷新到磁盘
//! 4. **回滚**: 简单地丢弃修改（依赖 cache 的 dirty flag 清除）
//!
//! ## 与完整 Journal 的对比
//!
//! | 特性 | SimpleTransaction | JournalTransaction |
//! |------|------------------|-------------------|
//! | 崩溃恢复 | ❌ 无 | ✅ 完整支持 |
//! | 原子性 | ❌ 无保证 | ✅ 保证 |
//! | 性能 | ✅ 快（无 journal 开销）| ⚠️ 较慢（双写） |
//! | 实现复杂度 | ✅ 简单 | ❌ 复杂 |
//! | 生产环境 | ❌ 不适用 | ✅ 适用 |

use crate::{
    block::{Block, BlockDev, BlockDevice},
    error::{Error, ErrorKind, Result},
};
use alloc::vec::Vec;

/// 简化的事务系统
///
/// ⚠️ 不提供崩溃一致性保证，仅用于开发和测试
pub struct SimpleTransaction<'a, D: BlockDevice> {
    /// 块设备引用
    bdev: &'a mut BlockDev<D>,

    /// 在此事务中被修改的块列表
    dirty_blocks: Vec<u64>,

    /// 事务状态
    state: TransactionState,
}

/// 事务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionState {
    /// 事务活跃，可以进行修改
    Active,

    /// 事务正在提交
    Committing,

    /// 事务已提交
    Committed,

    /// 事务已回滚
    Aborted,
}

impl<'a, D: BlockDevice> SimpleTransaction<'a, D> {
    /// 开始新事务
    ///
    /// # 参数
    ///
    /// * `bdev` - 块设备引用
    ///
    /// # 返回
    ///
    /// 新的事务对象
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut trans = SimpleTransaction::begin(&mut bdev)?;
    /// // ... 执行修改 ...
    /// trans.commit()?;
    /// ```
    pub fn begin(bdev: &'a mut BlockDev<D>) -> Result<Self> {
        Ok(Self {
            bdev,
            dirty_blocks: Vec::new(),
            state: TransactionState::Active,
        })
    }

    /// 获取块用于读取或修改
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 返回
    ///
    /// Block 句柄
    ///
    /// # 注意
    ///
    /// 获取块后如果进行了修改，必须调用 `mark_dirty(lba)` 标记为脏
    pub fn get_block(&mut self, lba: u64) -> Result<Block<D>> {
        self.check_active()?;
        Block::get(self.bdev, lba)
    }

    /// 获取块但不从磁盘读取（用于新分配的块）
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 返回
    ///
    /// Block 句柄
    pub fn get_block_noread(&mut self, lba: u64) -> Result<Block<D>> {
        self.check_active()?;
        Block::get_noread(self.bdev, lba)
    }

    /// 标记块为脏（已修改）
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    ///
    /// # 注意
    ///
    /// 必须在修改块后调用此方法，否则修改不会被提交
    pub fn mark_dirty(&mut self, lba: u64) -> Result<()> {
        self.check_active()?;

        // 避免重复添加
        if !self.dirty_blocks.contains(&lba) {
            self.dirty_blocks.push(lba);
        }

        Ok(())
    }

    /// 提交事务
    ///
    /// 将所有修改的块刷新到磁盘。
    ///
    /// ⚠️ **注意**：这个实现不提供原子性保证！
    /// 如果在刷新过程中崩溃，可能导致部分修改被写入，部分未写入。
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())，失败返回错误
    ///
    /// # 错误
    ///
    /// - `InvalidState`: 事务不在活跃状态
    /// - IO 错误: 刷新块时的底层 IO 错误
    pub fn commit(mut self) -> Result<()> {
        self.check_active()?;
        self.state = TransactionState::Committing;

        // 刷新所有脏块到磁盘
        // ⚠️ 这里没有原子性保证：如果中间崩溃，可能导致部分写入
        for &lba in &self.dirty_blocks {
            if let Err(e) = self.bdev.flush_lba(lba) {
                // 刷新失败，回滚剩余块
                self.state = TransactionState::Aborted;
                return Err(Error::with_cause(
                    ErrorKind::Io,
                    "Failed to flush block during commit",
                    e,
                ));
            }
        }

        self.state = TransactionState::Committed;
        Ok(())
    }

    /// 回滚事务
    ///
    /// 丢弃所有修改，不写入磁盘。
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    ///
    /// # 注意
    ///
    /// 简化实现依赖于 block cache 的自动清理。
    /// 如果事务被 drop 而没有 commit，也会自动回滚。
    pub fn abort(self) -> Result<()> {
        // 内部方法处理 abort 逻辑
        // 此处简单消费 self，不返回错误
        // Drop 会处理实际的清理
        Ok(())
    }

    /// 内部 abort 实现（供 Drop 使用）
    fn abort_internal(&mut self) {
        if self.state == TransactionState::Active {
            self.state = TransactionState::Aborted;
            self.dirty_blocks.clear();
        }
    }

    /// 获取事务状态
    pub fn state(&self) -> TransactionState {
        self.state
    }

    /// 获取脏块数量
    pub fn dirty_count(&self) -> usize {
        self.dirty_blocks.len()
    }

    /// 检查事务是否活跃
    fn check_active(&self) -> Result<()> {
        if self.state != TransactionState::Active {
            return Err(Error::new(
                ErrorKind::InvalidState,
                "Transaction is not active",
            ));
        }
        Ok(())
    }

    /// 获取 BlockDev 引用（用于某些需要直接访问的操作）
    ///
    /// ⚠️ **警告**：直接使用 bdev 可能绕过事务跟踪！
    /// 仅在明确知道自己在做什么时使用。
    pub fn bdev(&mut self) -> &mut BlockDev<D> {
        self.bdev
    }
}

impl<'a, D: BlockDevice> Drop for SimpleTransaction<'a, D> {
    /// 自动回滚未提交的事务
    ///
    /// 如果事务对象被 drop 但还没有 commit，自动执行 abort。
    /// 这提供了类似 RAII 的安全性，防止忘记提交或回滚。
    fn drop(&mut self) {
        self.abort_internal();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：这里的测试需要 mock BlockDevice
    // 完整测试在集成测试中进行

    #[test]
    fn test_transaction_states() {
        // 测试状态转换逻辑
        let state = TransactionState::Active;
        assert_eq!(state, TransactionState::Active);
    }

    #[test]
    fn test_dirty_blocks_tracking() {
        // 测试脏块跟踪（需要 mock device）
        // 暂时跳过，在集成测试中完成
    }
}
