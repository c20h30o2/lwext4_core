//! Inode 操作模块
//!
//! 这个模块提供 ext4 inode 的读取、验证、写入和更新功能。
//! TODO:考虑inode模块与inode_ref模块的职责划分，可以适当重构,减少代码冗余
//! 另外，对于类似的功能，有些地方使用了inode模块提供的接口，有的地方使用了inode_ref提供的接口，
//! 考虑在更高层统一使用inode_ref提供的接口，并修改inode模块，使其为inode_ref提供基础支持

mod read;
mod write;
pub mod checksum;

pub use read::*;
pub use write::*;
