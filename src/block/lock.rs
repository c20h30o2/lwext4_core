//! 块设备并发锁接口
//!
//! 对应 lwext4 的 `ext4_block_dev_lock/unlock` API

use crate::error::Result;

/// 块设备锁 trait
///
/// 用于在多线程环境中保护块设备访问。
/// 实现此 trait 的类型可以作为 BlockDev 的锁提供者。
///
/// # 设计说明
///
/// 这是一个接口预留，允许用户根据需要选择锁实现：
/// - `NoLock` - 单线程环境，无锁（默认）
/// - `Mutex<()>` - 互斥锁
/// - `RwLock<()>` - 读写锁
/// - 自定义锁实现
///
/// # 示例
///
/// ```rust,ignore
/// // 单线程（无锁）
/// let block_dev = BlockDev::new(device)?;
///
/// // 多线程（Mutex）
/// let block_dev = BlockDev::with_lock(device, Mutex::new(()))?;
/// ```
pub trait DeviceLock: Send {
    /// 获取锁
    ///
    /// 对应 lwext4 的 `ext4_block_dev_lock()`
    fn lock(&self) -> Result<()>;

    /// 释放锁
    ///
    /// 对应 lwext4 的 `ext4_block_dev_unlock()`
    fn unlock(&self) -> Result<()>;
}

/// 无锁实现（默认）
///
/// 用于单线程环境或已知不需要并发保护的场景
pub struct NoLock;

impl DeviceLock for NoLock {
    #[inline]
    fn lock(&self) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn unlock(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(feature = "std")]
mod std_locks {
    use super::*;
    use std::sync::{Mutex, RwLock};

    /// Mutex 锁实现
    impl DeviceLock for Mutex<()> {
        fn lock(&self) -> Result<()> {
            let _ = self.lock().map_err(|_| {
                crate::error::Error::new(
                    crate::error::ErrorKind::Io,
                    "Failed to acquire mutex lock",
                )
            })?;
            Ok(())
        }

        fn unlock(&self) -> Result<()> {
            // Rust 的 Mutex 在 lock guard drop 时自动释放
            // 这里不需要显式 unlock
            Ok(())
        }
    }

    /// RwLock 写锁实现
    impl DeviceLock for RwLock<()> {
        fn lock(&self) -> Result<()> {
            let _ = self.write().map_err(|_| {
                crate::error::Error::new(
                    crate::error::ErrorKind::Io,
                    "Failed to acquire write lock",
                )
            })?;
            Ok(())
        }

        fn unlock(&self) -> Result<()> {
            // Rust 的 RwLock 在 lock guard drop 时自动释放
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_lock() {
        let lock = NoLock;
        lock.lock().unwrap();
        lock.unlock().unwrap();
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_mutex_lock() {
        use std::sync::Mutex;

        let lock = Mutex::new(());
        lock.lock().unwrap();
        lock.unlock().unwrap();
    }
}
