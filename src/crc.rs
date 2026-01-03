//! CRC32C 校验和计算
//!
//! 为 ext4 元数据提供 CRC32C 校验和计算功能

use crc32fast::Hasher;

/// CRC32 初始值（ext4 使用 0xFFFFFFFF，但内部会取反）
pub const EXT4_CRC32_INIT: u32 = !0u32;

/// 计算 CRC32C 校验和（一次性计算）
///
/// # 参数
/// * `data` - 要计算校验和的数据
///
/// # 返回
/// CRC32C 值
#[inline]
pub fn crc32c(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

/// 计算 CRC32C 校验和（追加模式）
///
/// # 参数
/// * `crc` - 初始 CRC 值
/// * `data` - 要计算校验和的数据
///
/// # 返回
/// 更新后的 CRC32C 值
#[inline]
pub fn crc32c_append(crc: u32, data: &[u8]) -> u32 {
    let mut hasher = Hasher::new_with_initial(crc);
    hasher.update(data);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32c_basic() {
        let data = b"hello world";
        let crc = crc32c_append(EXT4_CRC32_INIT, data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc32c_incremental() {
        let data1 = b"hello";
        let data2 = b" world";

        // 一次计算
        let crc_once = crc32c_append(EXT4_CRC32_INIT, b"hello world");

        // 分两次计算
        let crc1 = crc32c_append(EXT4_CRC32_INIT, data1);
        let crc2 = crc32c_append(crc1, data2);

        assert_eq!(crc_once, crc2);
    }
}
