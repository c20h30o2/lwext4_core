//! ArceOS 集成所需的类型定义
//!
//! 这个模块定义了与 lwext4_rust 兼容的类型，用于 ArceOS 文件系统集成

use crate::consts::*;
use core::time::Duration;

/// 系统硬件抽象层 trait
///
/// 提供文件系统所需的系统级功能，主要是时间戳支持
pub trait SystemHal {
    /// 获取当前系统时间
    ///
    /// # 返回
    ///
    /// - `Some(Duration)` - 当前时间（从 UNIX 纪元开始）
    /// - `None` - 时间不可用（例如在没有RTC的嵌入式系统中）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// struct MyHal;
    /// impl SystemHal for MyHal {
    ///     fn now() -> Option<Duration> {
    ///         Some(Duration::from_secs(get_unix_timestamp()))
    ///     }
    /// }
    /// ```
    fn now() -> Option<Duration>;
}

/// 文件系统配置
#[derive(Debug, Clone, Copy)]
pub struct FsConfig {
    /// 块缓存大小（块数）
    pub bcache_size: u32,
}

impl Default for FsConfig {
    fn default() -> Self {
        Self {
            bcache_size: 256, // 默认 256 个块
        }
    }
}

/// 文件系统统计信息
#[derive(Debug, Clone, Copy, Default)]
pub struct StatFs {
    /// 总 inode 数
    pub inodes_count: u32,
    /// 空闲 inode 数
    pub free_inodes_count: u32,
    /// 总块数
    pub blocks_count: u64,
    /// 空闲块数
    pub free_blocks_count: u64,
    /// 块大小（字节）
    pub block_size: u32,
}

/// 文件属性
#[derive(Debug, Clone, Copy, Default)]
pub struct FileAttr {
    /// 设备 ID
    pub device: u64,
    /// 硬链接数
    pub nlink: u32,
    /// 文件模式（权限 + 类型）
    pub mode: u32,
    /// Inode 类型
    pub node_type: InodeType,
    /// 用户 ID
    pub uid: u32,
    /// 组 ID
    pub gid: u32,
    /// 文件大小（字节）
    pub size: u64,
    /// 块大小
    pub block_size: u64,
    /// 占用的块数
    pub blocks: u64,
    /// 访问时间（秒）
    pub atime: u64,
    /// 修改时间（秒）
    pub mtime: u64,
    /// 状态改变时间（秒）
    pub ctime: u64,
}

/// Inode 类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum InodeType {
    /// 未知类型
    #[default]
    Unknown = 0,
    /// FIFO（命名管道）
    Fifo = 1,
    /// 字符设备
    CharacterDevice = 2,
    /// 目录
    Directory = 3,
    /// 块设备
    BlockDevice = 4,
    /// 普通文件
    RegularFile = 5,
    /// 符号链接
    Symlink = 6,
    /// Socket
    Socket = 7,
}

impl InodeType {
    /// 从 mode 中提取 inode 类型
    pub fn from_mode(mode: u32) -> Self {
        let type_bits = (mode as u16) & EXT4_INODE_MODE_TYPE_MASK;
        match type_bits {
            EXT4_INODE_MODE_FIFO => InodeType::Fifo,
            EXT4_INODE_MODE_CHARDEV => InodeType::CharacterDevice,
            EXT4_INODE_MODE_DIRECTORY => InodeType::Directory,
            EXT4_INODE_MODE_BLOCKDEV => InodeType::BlockDevice,
            EXT4_INODE_MODE_FILE => InodeType::RegularFile,
            EXT4_INODE_MODE_SOFTLINK => InodeType::Symlink,
            EXT4_INODE_MODE_SOCKET => InodeType::Socket,
            _ => InodeType::Unknown,
        }
    }

    /// 转换为 mode 类型位
    pub fn to_mode_bits(self) -> u16 {
        match self {
            InodeType::Fifo => EXT4_INODE_MODE_FIFO,
            InodeType::CharacterDevice => EXT4_INODE_MODE_CHARDEV,
            InodeType::Directory => EXT4_INODE_MODE_DIRECTORY,
            InodeType::BlockDevice => EXT4_INODE_MODE_BLOCKDEV,
            InodeType::RegularFile => EXT4_INODE_MODE_FILE,
            InodeType::Symlink => EXT4_INODE_MODE_SOFTLINK,
            InodeType::Socket => EXT4_INODE_MODE_SOCKET,
            InodeType::Unknown => 0,
        }
    }

    /// 从 ext4 目录条目类型转换
    pub fn from_de_type(de_type: u8) -> Self {
        match de_type {
            1 => InodeType::RegularFile,
            2 => InodeType::Directory,
            3 => InodeType::CharacterDevice,
            4 => InodeType::BlockDevice,
            5 => InodeType::Fifo,
            6 => InodeType::Socket,
            7 => InodeType::Symlink,
            _ => InodeType::Unknown,
        }
    }

    /// 转换为 ext4 目录条目类型
    pub fn to_de_type(self) -> u8 {
        match self {
            InodeType::RegularFile => 1,
            InodeType::Directory => 2,
            InodeType::CharacterDevice => 3,
            InodeType::BlockDevice => 4,
            InodeType::Fifo => 5,
            InodeType::Socket => 6,
            InodeType::Symlink => 7,
            InodeType::Unknown => 0,
        }
    }

    /// 检查是否为目录
    pub fn is_dir(self) -> bool {
        self == InodeType::Directory
    }

    /// 检查是否为普通文件
    pub fn is_file(self) -> bool {
        self == InodeType::RegularFile
    }

    /// 检查是否为符号链接
    pub fn is_symlink(self) -> bool {
        self == InodeType::Symlink
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inode_type_from_mode() {
        assert_eq!(
            InodeType::from_mode(EXT4_INODE_MODE_FILE as u32),
            InodeType::RegularFile
        );
        assert_eq!(
            InodeType::from_mode(EXT4_INODE_MODE_DIRECTORY as u32),
            InodeType::Directory
        );
        assert_eq!(
            InodeType::from_mode(EXT4_INODE_MODE_SOFTLINK as u32),
            InodeType::Symlink
        );
    }

    #[test]
    fn test_inode_type_to_mode_bits() {
        assert_eq!(InodeType::RegularFile.to_mode_bits(), EXT4_INODE_MODE_FILE);
        assert_eq!(InodeType::Directory.to_mode_bits(), EXT4_INODE_MODE_DIRECTORY);
        assert_eq!(InodeType::Symlink.to_mode_bits(), EXT4_INODE_MODE_SOFTLINK);
    }

    #[test]
    fn test_inode_type_de_conversion() {
        assert_eq!(InodeType::from_de_type(1), InodeType::RegularFile);
        assert_eq!(InodeType::from_de_type(2), InodeType::Directory);
        assert_eq!(InodeType::from_de_type(7), InodeType::Symlink);

        assert_eq!(InodeType::RegularFile.to_de_type(), 1);
        assert_eq!(InodeType::Directory.to_de_type(), 2);
        assert_eq!(InodeType::Symlink.to_de_type(), 7);
    }

    #[test]
    fn test_inode_type_checks() {
        assert!(InodeType::Directory.is_dir());
        assert!(!InodeType::RegularFile.is_dir());

        assert!(InodeType::RegularFile.is_file());
        assert!(!InodeType::Directory.is_file());

        assert!(InodeType::Symlink.is_symlink());
        assert!(!InodeType::RegularFile.is_symlink());
    }

    #[test]
    fn test_fs_config_default() {
        let config = FsConfig::default();
        assert_eq!(config.bcache_size, 256);
    }
}
