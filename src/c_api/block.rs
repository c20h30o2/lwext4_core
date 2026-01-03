//! C API 兼容层 - 块操作
//!
//! 提供 lwext4 C 库兼容的函数名，仅在命名上保留 C 风格。
//! 所有函数都是 Rust 方法的简单包装。

use crate::{block::{BlockDev, BlockDevice}, Result};

/// C API: ext4_blocks_get_direct
///
/// 直接读取块。
///
/// 此函数提供与 lwext4 C 库兼容的接口。
/// 内部调用 `BlockDev::read_block`。
pub fn ext4_blocks_get_direct<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    lba: u64,
    buf: &mut [u8],
) -> Result<usize> {
    bdev.read_block(lba, buf)
}

/// C API: ext4_blocks_set_direct
///
/// 直接写入块。
///
/// 此函数提供与 lwext4 C 库兼容的接口。
/// 内部调用 `BlockDev::write_block`。
pub fn ext4_blocks_set_direct<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    lba: u64,
    buf: &[u8],
) -> Result<usize> {
    bdev.write_block(lba, buf)
}

/// C API: ext4_block_readbytes
///
/// 按字节偏移读取数据。
///
/// 此函数提供与 lwext4 C 库兼容的接口。
/// 内部调用 `BlockDev::read_bytes`。
pub fn ext4_block_readbytes<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    offset: u64,
    buf: &mut [u8],
) -> Result<usize> {
    bdev.read_bytes(offset, buf)
}

/// C API: ext4_block_writebytes
///
/// 按字节偏移写入数据。
///
/// 此函数提供与 lwext4 C 库兼容的接口。
/// 内部调用 `BlockDev::write_bytes`。
pub fn ext4_block_writebytes<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    offset: u64,
    buf: &[u8],
) -> Result<usize> {
    bdev.write_bytes(offset, buf)
}

/// C API: ext4_block_cache_flush
///
/// 刷新块缓存。
///
/// 此函数提供与 lwext4 C 库兼容的接口。
/// 内部调用 `BlockDev::flush`。
pub fn ext4_block_cache_flush<D: BlockDevice>(bdev: &mut BlockDev<D>) -> Result<()> {
    bdev.flush()
}

// 以下是占位函数，暂时不需要实现

/// C API: ext4_block_init (占位)
pub fn ext4_block_init<D: BlockDevice>(_bdev: &mut BlockDev<D>) -> Result<()> {
    Ok(())
}

/// C API: ext4_block_fini (占位)
pub fn ext4_block_fini<D: BlockDevice>(_bdev: &mut BlockDev<D>) -> Result<()> {
    Ok(())
}
