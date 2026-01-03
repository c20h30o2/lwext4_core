//! 块位图校验和功能
//!
//! 对应 lwext4 的位图校验和相关功能

use crate::{
    consts::*,
    superblock::Superblock,
    types::ext4_group_desc,
};

/// 计算块位图的 CRC32 校验和
///
/// 对应 lwext4 的 `ext4_balloc_bitmap_csum()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `bitmap` - 位图数据
///
/// # 返回
///
/// CRC32 校验和值
#[cfg(feature = "metadata-csum")]
pub fn bitmap_csum(sb: &Superblock, bitmap: &[u8]) -> u32 {
    if !sb.has_ro_compat_feature(EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        return 0;
    }

    let blocks_per_group = sb.blocks_per_group();

    // 先计算 UUID 的校验和
    let mut csum = crate::crc::crc32c_append(0xFFFFFFFF, sb.uuid());

    // 然后计算位图的校验和
    let bitmap_size = ((blocks_per_group + 7) / 8) as usize;
    csum = crate::crc::crc32c_append(csum, &bitmap[..bitmap_size]);

    csum
}

/// 占位实现：当未启用 metadata-csum 功能时
#[cfg(not(feature = "metadata-csum"))]
pub fn bitmap_csum(_sb: &Superblock, _bitmap: &[u8]) -> u32 {
    0
}

/// 设置块位图的校验和
///
/// 对应 lwext4 的 `ext4_balloc_set_bitmap_csum()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `bg` - 块组描述符引用
/// * `bitmap` - 位图数据
pub fn set_bitmap_csum(sb: &Superblock, bg: &mut ext4_group_desc, bitmap: &[u8]) {
    if !sb.has_ro_compat_feature(EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        return;
    }

    let csum = bitmap_csum(sb, bitmap);
    let lo_csum = (csum & 0xFFFF) as u16;
    let hi_csum = ((csum >> 16) & 0xFFFF) as u16;

    // 设置低 16 位
    bg.block_bitmap_csum_lo = lo_csum.to_le();

    // 如果是 64 位描述符，设置高 16 位
    if sb.group_desc_size() == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
        bg.block_bitmap_csum_hi = hi_csum.to_le();
    }
}

/// 验证块位图的校验和
///
/// 对应 lwext4 的 `ext4_balloc_verify_bitmap_csum()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `bg` - 块组描述符引用
/// * `bitmap` - 位图数据
///
/// # 返回
///
/// 校验和是否正确
#[cfg(feature = "metadata-csum")]
pub fn verify_bitmap_csum(sb: &Superblock, bg: &ext4_group_desc, bitmap: &[u8]) -> bool {
    if !sb.has_ro_compat_feature(EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        return true;
    }

    let csum = bitmap_csum(sb, bitmap);
    let lo_csum = (csum & 0xFFFF) as u16;
    let hi_csum = ((csum >> 16) & 0xFFFF) as u16;

    // 验证低 16 位
    if u16::from_le(bg.block_bitmap_csum_lo) != lo_csum {
        return false;
    }

    // 如果是 64 位描述符，验证高 16 位
    if sb.group_desc_size() == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
        if u16::from_le(bg.block_bitmap_csum_hi) != hi_csum {
            return false;
        }
    }

    true
}

/// 占位实现：当未启用 metadata-csum 功能时，总是返回 true
#[cfg(not(feature = "metadata-csum"))]
pub fn verify_bitmap_csum(_sb: &Superblock, _bg: &ext4_group_desc, _bitmap: &[u8]) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_bitmap_csum_disabled() {
        // 当未启用 metadata-csum 特性时，应该总是返回 0 或 true
        #[cfg(not(feature = "metadata-csum"))]
        {
            let bitmap = vec![0u8; 1024];
            let mut sb_inner = crate::types::ext4_sblock::default();
            let sb = Superblock::new(sb_inner);

            assert_eq!(bitmap_csum(&sb, &bitmap), 0);
        }
    }
}
