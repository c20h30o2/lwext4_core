//! Inode 校验和计算
//!
//! 对应 lwext4 的 `ext4_inode_get_csum()` 和 `ext4_inode_set_csum()` 功能

use crate::{
    consts::{EXT4_FEATURE_RO_COMPAT_METADATA_CSUM, EXT4_GOOD_OLD_INODE_SIZE},
    superblock::Superblock,
    types::ext4_inode,
    crc::EXT4_CRC32_INIT,
};

/// 获取 inode 校验和
///
/// 对应 lwext4 的 `ext4_inode_get_csum()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode` - inode 结构引用
///
/// # 返回
///
/// 32 位校验和值
pub fn get_checksum(sb: &Superblock, inode: &ext4_inode) -> u32 {
    let inode_size = sb.inode_size();
    let mut v = u16::from_le(inode.checksum_lo) as u32;

    if inode_size > EXT4_GOOD_OLD_INODE_SIZE as u16 {
        v |= (u16::from_le(inode.checksum_hi) as u32) << 16;
    }

    v
}

/// 设置 inode 校验和
///
/// 对应 lwext4 的 `ext4_inode_set_csum()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode` - 可变 inode 结构引用
/// * `checksum` - 校验和值
pub fn set_checksum(sb: &Superblock, inode: &mut ext4_inode, checksum: u32) {
    let inode_size = sb.inode_size();
    inode.checksum_lo = (checksum as u16).to_le();

    if inode_size > EXT4_GOOD_OLD_INODE_SIZE as u16 {
        inode.checksum_hi = ((checksum >> 16) as u16).to_le();
    }
}

/// 计算 inode 的 CRC32C 校验和
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode_num` - inode 编号
/// * `inode` - inode 结构引用
///
/// # 返回
///
/// 32 位 CRC32C 校验和
pub fn compute_checksum(sb: &Superblock, inode_num: u32, inode: &ext4_inode) -> u32 {
    let inode_size = sb.inode_size() as usize;

    // 将 inode 转换为字节切片
    let inode_bytes = unsafe {
        core::slice::from_raw_parts(
            inode as *const ext4_inode as *const u8,
            inode_size,
        )
    };

    // 初始化 CRC，包含 inode 编号
    let inode_num_bytes = inode_num.to_le_bytes();
    let mut crc = crate::crc::crc32c_append(EXT4_CRC32_INIT, &inode_num_bytes);

    // 计算 checksum_lo 字段的偏移量
    let checksum_lo_offset = offset_of_checksum_lo();

    // 计算到 checksum_lo 之前的数据
    if checksum_lo_offset > 0 {
        crc = crate::crc::crc32c_append(crc, &inode_bytes[..checksum_lo_offset]);
    }

    // checksum_lo 字段应该被视为0（2字节）
    let zero_bytes = [0u8; 2];
    crc = crate::crc::crc32c_append(crc, &zero_bytes);

    let after_checksum_lo = checksum_lo_offset + 2;

    if inode_size > EXT4_GOOD_OLD_INODE_SIZE {
        // 对于扩展 inode，需要处理 checksum_hi 字段
        let checksum_hi_offset = offset_of_checksum_hi();

        // 计算 checksum_lo 之后到 checksum_hi 之前的数据
        if checksum_hi_offset > after_checksum_lo {
            crc = crate::crc::crc32c_append(crc, &inode_bytes[after_checksum_lo..checksum_hi_offset]);
        }

        // checksum_hi 字段应该被视为0（2字节）
        crc = crate::crc::crc32c_append(crc, &zero_bytes);

        // 计算 checksum_hi 之后的数据
        let after_checksum_hi = checksum_hi_offset + 2;
        if inode_size > after_checksum_hi {
            crc = crate::crc::crc32c_append(crc, &inode_bytes[after_checksum_hi..inode_size]);
        }
    } else {
        // 对于标准 inode，计算 checksum_lo 之后的所有数据
        if inode_size > after_checksum_lo {
            crc = crate::crc::crc32c_append(crc, &inode_bytes[after_checksum_lo..inode_size]);
        }
    }

    crc
}

/// 验证 inode 校验和
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode_num` - inode 编号
/// * `inode` - inode 结构引用
///
/// # 返回
///
/// 如果校验和正确或不需要校验返回 `true`
pub fn verify_checksum(sb: &Superblock, inode_num: u32, inode: &ext4_inode) -> bool {
    // 检查是否启用了 METADATA_CSUM 特性
    if !sb.has_ro_compat_feature(EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        return true;
    }

    let stored = get_checksum(sb, inode);
    let computed = compute_checksum(sb, inode_num, inode);

    stored == computed
}

/// 获取 checksum_lo 字段的偏移量
///
/// 在 ext4_inode 结构中的位置
fn offset_of_checksum_lo() -> usize {
    // checksum_lo 在偏移 124（根据 types.rs）
    124
}

/// 获取 checksum_hi 字段的偏移量
///
/// 在扩展 ext4_inode 结构中的位置
fn offset_of_checksum_hi() -> usize {
    // checksum_hi 在标准 inode 之后的扩展部分
    // 通常在偏移 130
    130
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ext4_sblock;

    #[test]
    fn test_checksum_without_feature() {
        let mut sb = ext4_sblock::default();
        sb.magic = crate::consts::EXT4_SUPERBLOCK_MAGIC.to_le();
        sb.inode_size = 128u16.to_le();
        sb.feature_ro_compat = 0u32.to_le();

        let superblock = Superblock::new(sb);
        let inode = ext4_inode::default();

        // 未启用 METADATA_CSUM，应该总是验证通过
        assert!(verify_checksum(&superblock, 1, &inode));
    }

    #[test]
    fn test_checksum_with_feature() {
        let mut sb = ext4_sblock::default();
        sb.magic = crate::consts::EXT4_SUPERBLOCK_MAGIC.to_le();
        sb.inode_size = 256u16.to_le();
        sb.feature_ro_compat = EXT4_FEATURE_RO_COMPAT_METADATA_CSUM.to_le();

        let superblock = Superblock::new(sb);
        let mut inode = ext4_inode::default();

        // 计算并设置校验和
        let csum = compute_checksum(&superblock, 1, &inode);
        set_checksum(&superblock, &mut inode, csum);

        // 应该验证通过
        assert!(verify_checksum(&superblock, 1, &inode));
    }

    #[test]
    fn test_checksum_corruption() {
        let mut sb = ext4_sblock::default();
        sb.magic = crate::consts::EXT4_SUPERBLOCK_MAGIC.to_le();
        sb.inode_size = 256u16.to_le();
        sb.feature_ro_compat = EXT4_FEATURE_RO_COMPAT_METADATA_CSUM.to_le();

        let superblock = Superblock::new(sb);
        let mut inode = ext4_inode::default();

        // 设置正确的校验和
        let csum = compute_checksum(&superblock, 1, &inode);
        set_checksum(&superblock, &mut inode, csum);

        // 验证应该通过
        assert!(verify_checksum(&superblock, 1, &inode));

        // 修改 inode 数据（模拟损坏）
        inode.size_lo = 12345u32.to_le();

        // 现在验证应该失败
        assert!(!verify_checksum(&superblock, 1, &inode));
    }

    #[test]
    fn test_checksum_get_set() {
        let mut sb = ext4_sblock::default();
        sb.magic = crate::consts::EXT4_SUPERBLOCK_MAGIC.to_le();
        sb.inode_size = 256u16.to_le();

        let superblock = Superblock::new(sb);
        let mut inode = ext4_inode::default();

        // 设置校验和
        set_checksum(&superblock, &mut inode, 0x12345678);

        // 读取应该相同
        assert_eq!(get_checksum(&superblock, &inode), 0x12345678);
    }
}
