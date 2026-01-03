//! 目录操作模块
//!
//! 这个模块提供 ext4 目录的解析和路径查找功能。
//!
//! ## 模块结构
//!
//! - `checksum` - 目录校验和功能（✅ 已完成，设计优秀）
//! - `iterator` - 目录迭代器（✅ 新实现，使用 Block handle）
//! - `path_lookup` - 路径查找（✅ 新实现，使用 InodeRef）
//! - `hash` - HTree 哈希算法（✅ 新实现，完整支持所有哈希版本）
//! - `htree` - HTree 索引功能（✅ 查找完成，写入部分完成）
//! - `write` - 目录写操作（✅ 新实现，支持添加/删除条目）
//! - `entry` - 旧的目录迭代器实现（⚠️ 已废弃，保留用于向后兼容）
//! - `lookup` - 旧的路径查找实现（⚠️ 已废弃，保留用于向后兼容）
//!
//! ## 使用建议
//!
//! **新代码应使用**：
//! - `iterator::DirIterator` - 新的迭代器
//! - `iterator::read_dir()` - 读取目录
//! - `reader::DirReader` - ArceOS 兼容的目录读取器
//! - `path_lookup::PathLookup` - 路径查找
//! - `path_lookup::lookup_path()` - 便捷函数
//!
//! **旧代码向后兼容**（不推荐）：
//! - `entry::DirIterator` - 旧的迭代器（使用 Vec 拷贝）
//! - `lookup::PathLookup` - 旧的查找器（使用 Inode::load）

// 新实现（推荐使用）
pub mod checksum;
pub mod iterator;
pub mod reader;
pub mod path_lookup;
pub mod hash;
pub mod htree;
pub mod write;

// 旧实现（向后兼容，已废弃）
#[deprecated(since = "0.2.0", note = "Use `iterator` module instead")]
mod entry;
#[deprecated(since = "0.2.0", note = "Use `path_lookup` module instead")]
mod lookup;

// 重新导出常用类型（新实现）
pub use iterator::{DirEntry, DirIterator, read_dir};
pub use reader::DirReader;
pub use path_lookup::{PathLookup, lookup_path, get_inode_ref_by_path};

// 向后兼容：重新导出旧 API（使用类型别名避免冲突）
#[allow(deprecated)]
pub use entry::{
    DirEntry as OldDirEntry,
    DirIterator as OldDirIterator,
    read_dir as old_read_dir,
};

#[allow(deprecated)]
pub use lookup::{
    PathLookup as OldPathLookup,
    lookup_path as old_lookup_path,
    get_inode_by_path as old_get_inode_by_path,
};
