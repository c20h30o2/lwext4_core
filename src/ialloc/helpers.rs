//! Inode 分配辅助函数

use crate::superblock::Superblock;

/// 将 inode 号转换为块组内的相对索引
///
/// 对应 lwext4 的 `ext4_ialloc_inode_to_bgidx()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode` - inode 编号
///
/// # 返回
///
/// 块组内的相对索引
pub fn inode_to_bgidx(sb: &Superblock, inode: u32) -> u32 {
    let inodes_per_group = sb.inodes_per_group();
    (inode - 1) % inodes_per_group
}

/// 将块组内的索引转换为绝对 inode 号
///
/// 对应 lwext4 的 `ext4_ialloc_bgidx_to_inode()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `index` - 块组内的索引
/// * `bgid` - 块组编号
///
/// # 返回
///
/// 绝对 inode 编号
pub fn bgidx_to_inode(sb: &Superblock, index: u32, bgid: u32) -> u32 {
    let inodes_per_group = sb.inodes_per_group();
    bgid * inodes_per_group + (index + 1)
}

/// 根据 inode 号计算所属块组编号
///
/// 对应 lwext4 的 `ext4_ialloc_get_bgid_of_inode()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode` - inode 编号
///
/// # 返回
///
/// 块组编号
pub fn get_bgid_of_inode(sb: &Superblock, inode: u32) -> u32 {
    let inodes_per_group = sb.inodes_per_group();
    (inode - 1) / inodes_per_group
}

/// 计算指定块组中的 inode 数量
///
/// 对应 lwext4 的 `ext4_inodes_in_group_cnt()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `bgid` - 块组编号
///
/// # 返回
///
/// 该块组中的 inode 数量
pub fn inodes_in_group_cnt(sb: &Superblock, bgid: u32) -> u32 {
    let inodes_per_group = sb.inodes_per_group();
    let total_inodes = sb.inodes_count();
    let block_group_count = sb.block_group_count();

    // 最后一个块组可能不满
    if bgid < block_group_count - 1 {
        inodes_per_group
    } else {
        // 最后一个块组
        let remaining = total_inodes % inodes_per_group;
        if remaining == 0 {
            inodes_per_group
        } else {
            remaining
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ext4_sblock;

    #[test]
    fn test_inode_to_bgidx() {
        let mut sb_inner = ext4_sblock::default();
        sb_inner.inodes_per_group = 8192u32.to_le();
        let sb = Superblock::new(sb_inner);

        // 第一个 inode (inode 1)
        assert_eq!(inode_to_bgidx(&sb, 1), 0);

        // 最后一个 inode in first group (inode 8192)
        assert_eq!(inode_to_bgidx(&sb, 8192), 8191);

        // 第二个块组的第一个 inode (inode 8193)
        assert_eq!(inode_to_bgidx(&sb, 8193), 0);
    }

    #[test]
    fn test_bgidx_to_inode() {
        let mut sb_inner = ext4_sblock::default();
        sb_inner.inodes_per_group = 8192u32.to_le();
        let sb = Superblock::new(sb_inner);

        // 第一个块组的第一个 inode
        assert_eq!(bgidx_to_inode(&sb, 0, 0), 1);

        // 第一个块组的最后一个 inode
        assert_eq!(bgidx_to_inode(&sb, 8191, 0), 8192);

        // 第二个块组的第一个 inode
        assert_eq!(bgidx_to_inode(&sb, 0, 1), 8193);
    }

    #[test]
    fn test_get_bgid_of_inode() {
        let mut sb_inner = ext4_sblock::default();
        sb_inner.inodes_per_group = 8192u32.to_le();
        let sb = Superblock::new(sb_inner);

        // inode 1 在第 0 个块组
        assert_eq!(get_bgid_of_inode(&sb, 1), 0);

        // inode 8192 在第 0 个块组
        assert_eq!(get_bgid_of_inode(&sb, 8192), 0);

        // inode 8193 在第 1 个块组
        assert_eq!(get_bgid_of_inode(&sb, 8193), 1);
    }

    #[test]
    fn test_roundtrip() {
        let mut sb_inner = ext4_sblock::default();
        sb_inner.inodes_per_group = 8192u32.to_le();
        let sb = Superblock::new(sb_inner);

        for inode in [1u32, 100, 8192, 8193, 16384, 16385] {
            let bgid = get_bgid_of_inode(&sb, inode);
            let index = inode_to_bgidx(&sb, inode);
            let reconstructed = bgidx_to_inode(&sb, index, bgid);
            assert_eq!(inode, reconstructed, "Roundtrip failed for inode {}", inode);
        }
    }
}
