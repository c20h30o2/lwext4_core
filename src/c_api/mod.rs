//! C API 兼容层
//!
//! 提供与 lwext4 C 库兼容的函数接口。
//!
//! 这些函数仅保留 C 风格的命名（`ext4_*`），内部实现全部使用 Rust 风格的方法。

pub mod block;

// 可以根据需要添加其他模块
// pub mod fs;
// pub mod inode;
// pub mod dir;
