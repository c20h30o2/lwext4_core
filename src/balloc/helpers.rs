//! 块分配辅助函数

use crate::superblock::Superblock;

/// 从块地址计算块组 ID
///
/// 对应 lwext4 的 `ext4_balloc_get_bgid_of_block()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `baddr` - 绝对块地址
///
/// # 返回
///
/// 块组索引
pub fn get_bgid_of_block(sb: &Superblock, mut baddr: u64) -> u32 {
    if sb.first_data_block() != 0 && baddr != 0 {
        baddr -= 1;
    }

    (baddr / sb.blocks_per_group() as u64) as u32
}

/// 从块组 ID 计算起始块地址
///
/// 对应 lwext4 的 `ext4_balloc_get_block_of_bgid()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `bgid` - 块组索引
///
/// # 返回
///
/// 块地址
pub fn get_block_of_bgid(sb: &Superblock, bgid: u32) -> u64 {
    let mut baddr = 0u64;

    if sb.first_data_block() != 0 {
        baddr += 1;
    }

    baddr += bgid as u64 * sb.blocks_per_group() as u64;
    baddr
}

/// 从块组内索引和块组 ID 计算绝对块地址
///
/// 对应 lwext4 的 `ext4_fs_bg_idx_to_addr()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `idx_in_bg` - 块组内索引
/// * `bgid` - 块组 ID
///
/// # 返回
///
/// 绝对块地址
pub fn bg_idx_to_addr(sb: &Superblock, idx_in_bg: u32, bgid: u32) -> u64 {
    let bg_first = get_block_of_bgid(sb, bgid);
    bg_first + idx_in_bg as u64
}

/// 从绝对块地址计算块组内索引
///
/// 对应 lwext4 的 `ext4_fs_addr_to_idx_bg()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `baddr` - 绝对块地址
///
/// # 返回
///
/// 块组内索引
pub fn addr_to_idx_bg(sb: &Superblock, mut baddr: u64) -> u32 {
    if sb.first_data_block() != 0 && baddr != 0 {
        baddr -= 1;
    }

    (baddr % sb.blocks_per_group() as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{consts::*, types::ext4_sblock};

    #[test]
    fn test_bgid_of_block() {
        let mut sb = ext4_sblock::default();
        sb.magic = EXT4_SUPERBLOCK_MAGIC.to_le();
        sb.first_data_block = 0u32.to_le();
        sb.blocks_per_group = 8192u32.to_le();
        let superblock = Superblock::new(sb);

        assert_eq!(get_bgid_of_block(&superblock, 0), 0);
        assert_eq!(get_bgid_of_block(&superblock, 8191), 0);
        assert_eq!(get_bgid_of_block(&superblock, 8192), 1);
        assert_eq!(get_bgid_of_block(&superblock, 16384), 2);
    }

    #[test]
    fn test_block_of_bgid() {
        let mut sb = ext4_sblock::default();
        sb.magic = EXT4_SUPERBLOCK_MAGIC.to_le();
        sb.first_data_block = 0u32.to_le();
        sb.blocks_per_group = 8192u32.to_le();
        let superblock = Superblock::new(sb);

        assert_eq!(get_block_of_bgid(&superblock, 0), 0);
        assert_eq!(get_block_of_bgid(&superblock, 1), 8192);
        assert_eq!(get_block_of_bgid(&superblock, 2), 16384);
    }

    #[test]
    fn test_addr_conversions() {
        let mut sb = ext4_sblock::default();
        sb.magic = EXT4_SUPERBLOCK_MAGIC.to_le();
        sb.first_data_block = 0u32.to_le();
        sb.blocks_per_group = 8192u32.to_le();
        let superblock = Superblock::new(sb);

        // 测试往返转换
        let bgid = 1u32;
        let idx_in_bg = 100u32;

        let addr = bg_idx_to_addr(&superblock, idx_in_bg, bgid);
        assert_eq!(addr, 8192 + 100);

        let recovered_idx = addr_to_idx_bg(&superblock, addr);
        assert_eq!(recovered_idx, idx_in_bg);

        let recovered_bgid = get_bgid_of_block(&superblock, addr);
        assert_eq!(recovered_bgid, bgid);
    }
}
