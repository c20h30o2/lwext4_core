//! 文件元数据

use crate::{consts::*, inode::Inode};

/// 文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// 普通文件
    RegularFile,
    /// 目录
    Directory,
    /// 符号链接
    Symlink,
    /// 字符设备
    CharDevice,
    /// 块设备
    BlockDevice,
    /// FIFO（命名管道）
    Fifo,
    /// Socket
    Socket,
    /// 未知类型
    Unknown,
}

impl FileType {
    /// 从 inode 模式解析文件类型
    pub(crate) fn from_mode(mode: u16) -> Self {
        match mode & EXT4_INODE_MODE_TYPE_MASK {
            EXT4_INODE_MODE_FILE => FileType::RegularFile,
            EXT4_INODE_MODE_DIRECTORY => FileType::Directory,
            EXT4_INODE_MODE_SOFTLINK => FileType::Symlink,
            EXT4_INODE_MODE_CHARDEV => FileType::CharDevice,
            EXT4_INODE_MODE_BLOCKDEV => FileType::BlockDevice,
            EXT4_INODE_MODE_FIFO => FileType::Fifo,
            EXT4_INODE_MODE_SOCKET => FileType::Socket,
            _ => FileType::Unknown,
        }
    }

    /// 是否是目录
    pub fn is_dir(&self) -> bool {
        matches!(self, FileType::Directory)
    }

    /// 是否是普通文件
    pub fn is_file(&self) -> bool {
        matches!(self, FileType::RegularFile)
    }

    /// 是否是符号链接
    pub fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink)
    }
}

/// 文件元数据
///
/// 包含文件的所有属性信息
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// 文件类型
    pub file_type: FileType,
    /// 文件大小（字节）
    pub size: u64,
    /// Inode 编号
    pub inode_num: u32,
    /// 访问权限（Unix 权限位）
    pub permissions: u16,
    /// 用户 ID
    pub uid: u32,
    /// 组 ID
    pub gid: u32,
    /// 访问时间（Unix 时间戳）
    pub atime: i64,
    /// 修改时间（Unix 时间戳）
    pub mtime: i64,
    /// 创建时间（Unix 时间戳）
    pub ctime: i64,
    /// 硬链接数
    pub links_count: u16,
    /// 占用的块数（512 字节块）
    pub blocks_count: u64,
}

impl FileMetadata {
    /// 从 inode 创建元数据
    pub(crate) fn from_inode(inode: &Inode, inode_num: u32) -> Self {
        let mode = inode.mode();

        Self {
            file_type: FileType::from_mode(mode),
            size: inode.file_size(),
            inode_num,
            permissions: mode & 0o7777, // 提取权限位
            uid: inode.uid(),
            gid: inode.gid(),
            atime: inode.access_time() as i64,
            mtime: inode.modification_time() as i64,
            ctime: inode.change_time() as i64,
            links_count: inode.links_count(),
            blocks_count: inode.blocks_count(),
        }
    }

    /// 是否是目录
    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    /// 是否是普通文件
    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }

    /// 是否是符号链接
    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_from_mode() {
        assert_eq!(
            FileType::from_mode(EXT4_INODE_MODE_FILE),
            FileType::RegularFile
        );
        assert_eq!(
            FileType::from_mode(EXT4_INODE_MODE_DIRECTORY),
            FileType::Directory
        );
        assert_eq!(
            FileType::from_mode(EXT4_INODE_MODE_SOFTLINK),
            FileType::Symlink
        );
    }

    #[test]
    fn test_file_type_checks() {
        assert!(FileType::Directory.is_dir());
        assert!(!FileType::Directory.is_file());
        assert!(!FileType::Directory.is_symlink());

        assert!(!FileType::RegularFile.is_dir());
        assert!(FileType::RegularFile.is_file());
        assert!(!FileType::RegularFile.is_symlink());

        assert!(!FileType::Symlink.is_dir());
        assert!(!FileType::Symlink.is_file());
        assert!(FileType::Symlink.is_symlink());
    }
}
