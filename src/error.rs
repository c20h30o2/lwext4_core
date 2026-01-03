//! 错误类型定义
//!
//! 提供 ext4 文件系统操作的错误类型。

use core::fmt;

/// ext4 操作错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    message: &'static str,
}

/// 错误类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// I/O 错误
    Io,
    /// 无效参数
    InvalidInput,
    /// 文件系统损坏
    Corrupted,
    /// 权限错误
    PermissionDenied,
    /// 文件不存在
    NotFound,
    /// 已存在
    AlreadyExists,
    /// 空间不足
    NoSpace,
    /// 不支持的操作
    Unsupported,
    /// 设备忙
    Busy,
    /// 无效状态
    InvalidState,
    /// 目录非空
    NotEmpty,
}

impl Error {
    /// 创建新错误
    pub const fn new(kind: ErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }

    /// 创建带原因的错误（简化版，忽略 cause）
    ///
    /// 注意：在 no_std 环境下，cause 参数会被忽略
    pub fn with_cause(kind: ErrorKind, message: &'static str, _cause: impl core::fmt::Debug) -> Self {
        Self { kind, message }
    }

    /// 获取错误类型
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// 获取错误消息
    pub const fn message(&self) -> &'static str {
        self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

// Journal error conversion
impl From<crate::journal::JournalError> for Error {
    fn from(err: crate::journal::JournalError) -> Self {
        use crate::journal::JournalError;
        match err {
            JournalError::NoJournalInode => Error::new(ErrorKind::NotFound, "Journal inode not found"),
            JournalError::InvalidSuperblock => Error::new(ErrorKind::Corrupted, "Invalid journal superblock"),
            JournalError::UnsupportedFeature(_) => Error::new(ErrorKind::Unsupported, "Unsupported journal feature"),
            JournalError::RecoveryFailed => Error::new(ErrorKind::InvalidState, "Journal recovery failed"),
            JournalError::NoSpace => Error::new(ErrorKind::NoSpace, "Journal has no space"),
            JournalError::IoError => Error::new(ErrorKind::Io, "Journal I/O error"),
        }
    }
}

/// Result 类型别名
pub type Result<T> = core::result::Result<T, Error>;
