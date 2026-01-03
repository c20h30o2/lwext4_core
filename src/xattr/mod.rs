//! ext4 扩展属性 (Extended Attributes) 实现
//!
//! 这个模块提供完整的 ext4 扩展属性功能，允许在文件和目录上存储额外的元数据。
//!
//! # 扩展属性概述
//!
//! 扩展属性（xattr）以 name-value 对的形式存储，支持多个命名空间：
//! - **user.** - 用户自定义属性
//! - **system.** - 系统属性（如 ACL）
//! - **security.** - 安全标签（如 SELinux）
//! - **trusted.** - 可信属性
//!
//! # 存储位置
//!
//! ext4 支持两种存储位置：
//! 1. **Inode 内部** - 存储在 inode 的额外空间中（快速访问）
//! 2. **独立块** - 存储在独立的 xattr 块中（通过 inode->file_acl 指向）
//!
//! # 功能特性
//!
//! - ✅ 命名空间前缀解析
//! - ✅ inode 内部 xattr 操作
//! - ✅ xattr 块操作
//! - ✅ 块共享和引用计数（COW）
//! - ✅ 哈希计算
//! - ✅ CRC32C 校验和
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use lwext4_core::xattr;
//!
//! // 列出所有扩展属性
//! let mut list = Vec::new();
//! xattr::list(&inode_ref, &mut list)?;
//!
//! // 获取属性值
//! let mut buf = vec![0u8; 256];
//! let len = xattr::get(&inode_ref, "user.comment", &mut buf)?;
//!
//! // 设置属性
//! xattr::set(&inode_ref, "user.author", b"Alice")?;
//!
//! // 删除属性
//! xattr::remove(&inode_ref, "user.temp")?;
//! ```
//!
//! # 实现状态
//!
//! - ✅ 命名空间前缀（prefix.rs）- 100% 完成 + 6个测试
//! - ✅ 搜索和查找（search.rs）- 100% 完成 + 5个测试
//! - ✅ 哈希和校验和（hash.rs）- 100% 完成 + 3个测试
//! - ✅ inode 内部操作（ibody.rs）- 100% 完成 + 2个测试
//! - ✅ 块操作（block.rs）- 100% 完成 + 3个测试
//! - ✅ 写操作逻辑（write.rs）- 100% 完成 + 5个测试
//! - ✅ 公共 API（api.rs）- 完整实现（list/get/set/remove）
//!
//! **总体完成度**: 100% (核心功能完整)

mod prefix;
mod search;
mod hash;
mod ibody;
mod block;
mod write;
mod api;

pub use api::{list, get, set, remove};
pub use prefix::{extract_xattr_name, get_xattr_name_prefix};
