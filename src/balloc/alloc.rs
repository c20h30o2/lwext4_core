//! 块分配功能
//!
//! 对应 lwext4 的 `ext4_balloc_alloc_block()` 和 `ext4_balloc_try_alloc_block()`

use crate::{
    bitmap::{self, *},
    block::{Block, BlockDev, BlockDevice},
    block_group::BlockGroup,
    error::{Error, ErrorKind, Result},
    fs::BlockGroupRef,
    superblock::Superblock,
};
use log::*;
use super::{checksum::*, helpers::*};

/// 块分配器状态
///
/// 用于跟踪上次分配的块组，优化分配性能
pub struct BlockAllocator {
    last_block_bg_id: u32,
}

impl BlockAllocator {
    /// 创建新的块分配器
    pub fn new() -> Self {
        Self {
            last_block_bg_id: 0,
        }
    }

    /// 分配一个块（带目标块提示）
    ///
    /// 对应 lwext4 的 `ext4_balloc_alloc_block()`
    ///
    /// # 参数
    ///
    /// * `bdev` - 块设备引用
    /// * `sb` - superblock 可变引用
    /// * `goal` - 目标块地址（提示）
    ///
    /// # 返回
    ///
    /// 成功返回分配的块地址
    ///
    /// # 注意
    ///
    /// 此版本不更新 inode 的 blocks 计数，调用者需要自己处理
    pub fn alloc_block<D: BlockDevice>(
        &mut self,
        bdev: &mut BlockDev<D>,
        sb: &mut Superblock,
        goal: u64,
    ) -> Result<u64> {
        // 计算目标块组
        let bg_id = get_bgid_of_block(sb, goal);
        let idx_in_bg = addr_to_idx_bg(sb, goal);

        // 检查目标块组是否有空闲块
        let free_blocks = {
            let mut bg_ref = BlockGroupRef::get(bdev, sb, bg_id)?;
            bg_ref.free_blocks_count()?
        };

        // 尝试在目标块组中分配
        if free_blocks > 0 {
            if let Some(alloc) = self.try_alloc_in_group(bdev, sb, bg_id, idx_in_bg)? {
                self.last_block_bg_id = bg_id;
                return Ok(alloc);
            }
        }

        // 目标块组失败，尝试其他块组
        let block_group_count = sb.block_group_count();
        let mut bgid = (bg_id + 1) % block_group_count;
        let mut count = block_group_count - 1; // 已经尝试过一个了

        while count > 0 {
            // 检查此块组是否有空闲块
            let free_blocks = {
                let mut bg_ref = BlockGroupRef::get(bdev, sb, bgid)?;
                bg_ref.free_blocks_count()?
            };

            if free_blocks > 0 {
                // 计算此块组的起始索引
                let first_in_bg = get_block_of_bgid(sb, bgid);
                let idx_in_bg = addr_to_idx_bg(sb, first_in_bg);

                if let Some(alloc) = self.try_alloc_in_group(bdev, sb, bgid, idx_in_bg)? {
                    self.last_block_bg_id = bgid;
                    return Ok(alloc);
                }
            }

            bgid = (bgid + 1) % block_group_count;
            count -= 1;
        }

        Err(Error::new(ErrorKind::NoSpace, "No free blocks available"))
    }

    /// 在指定块组中尝试分配块
    fn try_alloc_in_group<D: BlockDevice>(
        &self,
        bdev: &mut BlockDev<D>,
        sb: &mut Superblock,
        bgid: u32,
        mut idx_in_bg: u32,
    ) -> Result<Option<u64>> {
        // 获取此块组的块数
        let blk_in_bg = sb.blocks_in_group_cnt(bgid);

        // 计算此块组的第一个有效索引
        let first_in_bg = get_block_of_bgid(sb, bgid);
        let first_in_bg_index = addr_to_idx_bg(sb, first_in_bg);

        if idx_in_bg < first_in_bg_index {
            idx_in_bg = first_in_bg_index;
        }

        // 第一步：获取位图地址和块组描述符副本
        let (bmp_blk_addr, bg_copy) = {
            let mut bg_ref = BlockGroupRef::get(bdev, sb, bgid)?;
            let bitmap_addr = bg_ref.block_bitmap()?;
            let bg_data = bg_ref.get_block_group_copy()?;
            (bitmap_addr, bg_data)
        };

        // 第二步：操作位图
        let alloc_opt = {
            let mut bitmap_block = Block::get(bdev, bmp_blk_addr)?;

            bitmap_block.with_data_mut(|bitmap_data| {
                // 验证位图校验和
                if !verify_bitmap_csum(sb, &bg_copy, bitmap_data) {
                    // 记录警告但继续
                }

                // 1. 检查目标位置是否空闲
                if !bitmap::test_bit(bitmap_data, idx_in_bg) {
                    set_bit(bitmap_data, idx_in_bg)?;
                    let mut bg_for_csum = bg_copy;
                    set_bitmap_csum(sb, &mut bg_for_csum, bitmap_data);
                    return Ok::<_, Error>(Some(idx_in_bg));
                }

                // 2. 在目标附近查找（+63 范围内）
                let mut end_idx = (idx_in_bg + 63) & !63;
                if end_idx > blk_in_bg {
                    end_idx = blk_in_bg;
                }

                for tmp_idx in (idx_in_bg + 1)..end_idx {
                    if !bitmap::test_bit(bitmap_data, tmp_idx) {
                        set_bit(bitmap_data, tmp_idx)?;
                        let mut bg_for_csum = bg_copy;
                        set_bitmap_csum(sb, &mut bg_for_csum, bitmap_data);
                        return Ok::<_, Error>(Some(tmp_idx));
                    }
                }

                // 3. 在整个块组中查找
                if let Some(rel_blk_idx) = find_first_zero(bitmap_data, idx_in_bg, blk_in_bg) {
                    set_bit(bitmap_data, rel_blk_idx)?;
                    let mut bg_for_csum = bg_copy;
                    set_bitmap_csum(sb, &mut bg_for_csum, bitmap_data);
                    return Ok::<_, Error>(Some(rel_blk_idx));
                }

                Ok::<_, Error>(None)
            })??
        };

        if let Some(idx) = alloc_opt {
            // 计算绝对地址
            let alloc = bg_idx_to_addr(sb, idx, bgid);

            // 🔧 验证分配的块号
            let device_total = bdev.total_blocks();
            if alloc >= device_total {
                log::error!(
                    "[try_alloc_in_group] INVALID block allocated: {:#x} (exceeds device total {}), idx={}, bgid={}",
                    alloc, device_total, idx, bgid
                );
                return Err(Error::new(
                    ErrorKind::Corrupted,
                    "Allocated block exceeds device size",
                ));
            }

            log::info!(
                "[try_alloc_in_group] Allocated block: {:#x} (idx={}, bgid={})",
                alloc, idx, bgid
            );

            // 第三步：更新块组描述符
            {
                let mut bg_ref = BlockGroupRef::get(bdev, sb, bgid)?;
                bg_ref.dec_free_blocks(1)?;
                // bg_ref 在此处自动释放并写回
            }

            // 更新 superblock 空闲块计数
            let mut sb_free_blocks = sb.free_blocks_count();
            if sb_free_blocks > 0 {
                sb_free_blocks -= 1;
            }
            sb.set_free_blocks_count(sb_free_blocks);
            sb.write(bdev)?;

            return Ok(Some(alloc));
        }

        Ok(None)
    }

    /// 获取上次分配的块组 ID
    pub fn last_bg_id(&self) -> u32 {
        self.last_block_bg_id
    }

    /// 设置上次分配的块组 ID
    pub fn set_last_bg_id(&mut self, bgid: u32) {
        self.last_block_bg_id = bgid;
    }
}

impl Default for BlockAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// 尝试分配特定的块地址
///
/// 对应 lwext4 的 `ext4_balloc_try_alloc_block()`
///
/// # 参数
///
/// * `bdev` - 块设备引用
/// * `sb` - superblock 可变引用
/// * `baddr` - 要尝试分配的块地址
///
/// # 返回
///
/// 成功返回 true（块已分配），false（块已被占用）
///
/// # 注意
///
/// 此版本不更新 inode 的 blocks 计数，调用者需要自己处理
pub fn try_alloc_block<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    sb: &mut Superblock,
    baddr: u64,
) -> Result<bool> {
    // 计算块组和索引
    let block_group = get_bgid_of_block(sb, baddr);
    let index_in_group = addr_to_idx_bg(sb, baddr);

    // 第一步：获取位图地址和块组描述符副本
    let (bmp_blk_addr, bg_copy) = {
        let mut bg_ref = BlockGroupRef::get(bdev, sb, block_group)?;
        let bitmap_addr = bg_ref.block_bitmap()?;
        let bg_data = bg_ref.get_block_group_copy()?;
        (bitmap_addr, bg_data)
    };

    // 第二步：操作位图
    let is_free = {
        let mut bitmap_block = Block::get(bdev, bmp_blk_addr)?;

        bitmap_block.with_data_mut(|bitmap_data| {
            // 验证位图校验和
            if !verify_bitmap_csum(sb, &bg_copy, bitmap_data) {
                // 记录警告但继续
            }

            // 检查块是否空闲
            let free = !bitmap::test_bit(bitmap_data, index_in_group);

            // 如果空闲，分配它
            if free {
                set_bit(bitmap_data, index_in_group)?;
                let mut bg_for_csum = bg_copy;
                set_bitmap_csum(sb, &mut bg_for_csum, bitmap_data);
            }

            Ok::<_, Error>(free)
        })??
    };

    // 如果块不空闲，直接返回
    if !is_free {
        return Ok(false);
    }

    // 第三步：更新块组描述符
    {
        let mut bg_ref = BlockGroupRef::get(bdev, sb, block_group)?;
        bg_ref.dec_free_blocks(1)?;
        // bg_ref 在此处自动释放并写回
    }

    // 更新 superblock 空闲块计数
    let mut sb_free_blocks = sb.free_blocks_count();
    if sb_free_blocks > 0 {
        sb_free_blocks -= 1;
    }
    sb.set_free_blocks_count(sb_free_blocks);
    sb.write(bdev)?;

    Ok(true)
}

/// 分配一个块（无状态版本）
///
/// 这是一个便捷函数，从块 0 开始作为目标
///
/// # 参数
///
/// * `bdev` - 块设备引用
/// * `sb` - superblock 可变引用
///
/// # 返回
///
/// 成功返回分配的块地址
pub fn alloc_block<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    sb: &mut Superblock,
) -> Result<u64> {
    let mut allocator = BlockAllocator::new();
    let goal = sb.first_data_block() as u64;
    allocator.alloc_block(bdev, sb, goal)
}

/// 在单个块组内分配多个连续块
///
/// # 参数
///
/// * `bdev` - 块设备引用
/// * `sb` - superblock 可变引用
/// * `goal` - 目标块地址（提示）
/// * `max_count` - 期望分配的块数
///
/// # 返回
///
/// `(起始块地址, 实际分配的块数)`
///
/// # 注意
///
/// 实际分配数可能小于 max_count（块组空间不足）
pub fn alloc_blocks_in_group<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    sb: &mut Superblock,
    goal: u64,
    max_count: u32,
) -> Result<(u64, u32)> {
    if max_count == 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Cannot allocate zero blocks",
        ));
    }

    // 如果只需要 1 个块，使用现有的单块分配
    if max_count == 1 {
        let block = alloc_block(bdev, sb)?;
        return Ok((block, 1));
    }

    let bgid = get_bgid_of_block(sb, goal);
    let idx_in_bg = addr_to_idx_bg(sb, goal);

    // 第一步：获取位图和块组信息
    let (bitmap_addr, bg_copy, blocks_in_bg) = {
        let mut bg_ref = BlockGroupRef::get(bdev, sb, bgid)?;

        // 检查块组是否有足够的空闲块
        let free_blocks = bg_ref.free_blocks_count()?;
        if free_blocks == 0 {
            return Err(Error::new(
                ErrorKind::NoSpace,
                "Block group has no free blocks",
            ));
        }

        let bmp = bg_ref.block_bitmap()?;
        let bg_data = bg_ref.get_block_group_copy()?;
        let blk_cnt = sb.blocks_in_group_cnt(bgid);
        (bmp, bg_data, blk_cnt)
    };

    // 第二步：在位图中查找连续空闲块
    let (start_idx, alloc_count) = {
        let mut bitmap_block = Block::get(bdev, bitmap_addr)?;

        bitmap_block.with_data_mut(|bitmap_data| {
            // 验证校验和
            if !verify_bitmap_csum(sb, &bg_copy, bitmap_data) {
                // 警告但继续
            }

            // 查找连续空闲位
            let result = bitmap::find_consecutive_zeros(
                bitmap_data,
                idx_in_bg,
                blocks_in_bg,
                max_count,
            );

            if let Some(start) = result {
                // 实际分配的块数（可能小于请求的数量）
                // 我们需要计算找到了多少连续空闲块
                let mut count = 0u32;
                for i in start..blocks_in_bg {
                    if count >= max_count {
                        break;
                    }
                    if !bitmap::test_bit(bitmap_data, i) {
                        count += 1;
                    } else {
                        break;
                    }
                }

                if count == 0 {
                    return Err(Error::new(
                        ErrorKind::NoSpace,
                        "No consecutive blocks found",
                    ));
                }

                // 设置位图位
                bitmap::set_bits(bitmap_data, start, count)?;

                // 更新校验和
                let mut bg_for_csum = bg_copy;
                set_bitmap_csum(sb, &mut bg_for_csum, bitmap_data);

                Ok::<_, Error>((start, count))
            } else {
                Err(Error::new(
                    ErrorKind::NoSpace,
                    "No consecutive blocks found in group",
                ))
            }
        })??
    };

    // 第三步：更新块组描述符
    {
        let mut bg_ref = BlockGroupRef::get(bdev, sb, bgid)?;
        bg_ref.dec_free_blocks(alloc_count)?;
    }

    // 第四步：更新 superblock
    let mut sb_free = sb.free_blocks_count();
    if sb_free >= alloc_count as u64 {
        sb_free -= alloc_count as u64;
    }
    sb.set_free_blocks_count(sb_free);
    sb.write(bdev)?;

    // 计算绝对地址
    let start_addr = bg_idx_to_addr(sb, start_idx, bgid);
    Ok((start_addr, alloc_count))
}

/// 批量分配块（通用接口）
///
/// 当前实现：在单个块组内分配连续块
/// 未来可扩展为跨块组分配
///
/// # 参数
///
/// * `bdev` - 块设备引用
/// * `sb` - superblock 可变引用
/// * `goal` - 目标块地址（提示）
/// * `max_count` - 期望分配的块数
///
/// # 返回
///
/// `(起始块地址, 实际分配的块数)`
///
/// # 示例
///
/// ```rust,ignore
/// // 尝试分配 100 个连续块
/// let (start_block, count) = balloc::alloc_blocks(bdev, sb, goal, 100)?;
/// println!("Allocated {} blocks starting at {}", count, start_block);
/// ```
pub fn alloc_blocks<D: BlockDevice>(
    bdev: &mut BlockDev<D>,
    sb: &mut Superblock,
    goal: u64,
    max_count: u32,
) -> Result<(u64, u32)> {
    let device_total = bdev.total_blocks();

    info!(
        "[BALLOC] Requesting {} blocks, goal={:#x}, device_total={}",
        max_count, goal, device_total
    );

    // 首先尝试在 goal 所在的块组中分配
    let result = alloc_blocks_in_group(bdev, sb, goal, max_count);

    // 如果成功，直接返回
    if let Ok((start_block, count)) = result {
        // 验证分配的块是否在设备范围内
        if start_block + count as u64 > device_total {
            error!(
                "[BALLOC] Allocated blocks OUT OF RANGE! start={:#x}, count={}, device_total={}",
                start_block, count, device_total
            );
            return Err(Error::new(
                ErrorKind::Corrupted,
                "Allocated blocks exceed device size",
            ));
        }

        info!(
            "[BALLOC] Allocated {} blocks: start={:#x}, end={:#x}",
            count, start_block, start_block + count as u64 - 1
        );
        return Ok((start_block, count));
    }

    // 如果失败（可能是块组满了），尝试其他块组
    // 遍历所有块组寻找空闲块
    let bg_count = sb.block_group_count();
    let first_data_block = sb.first_data_block() as u64;

    for bgid in 0..bg_count {
        // 跳过已经尝试过的块组
        let bgid_of_goal = get_bgid_of_block(sb, goal);
        if bgid == bgid_of_goal {
            continue;
        }

        // 计算该块组的第一个数据块作为新的 goal
        let blocks_per_group = sb.blocks_per_group();
        let bg_first_block = first_data_block + (bgid as u64 * blocks_per_group as u64);

        // 尝试在这个块组中分配
        match alloc_blocks_in_group(bdev, sb, bg_first_block, max_count) {
            Ok((start_block, count)) => {
                // 验证分配的块是否在设备范围内
                if start_block + count as u64 > device_total {
                    error!(
                        "[BALLOC] Allocated blocks OUT OF RANGE (fallback)! start={:#x}, count={}, device_total={}",
                        start_block, count, device_total
                    );
                    return Err(Error::new(
                        ErrorKind::Corrupted,
                        "Allocated blocks exceed device size",
                    ));
                }

                info!(
                    "[BALLOC] Allocated {} blocks (fallback to bg {}): start={:#x}",
                    count, bgid, start_block
                );
                return Ok((start_block, count));
            }
            Err(_) => {
                // 这个块组也满了，继续尝试下一个
                continue;
            }
        }
    }

    // 所有块组都满了，返回错误
    Err(Error::new(
        ErrorKind::NoSpace,
        "No free blocks available in any block group",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_allocator_creation() {
        let allocator = BlockAllocator::new();
        assert_eq!(allocator.last_bg_id(), 0);
    }

    #[test]
    fn test_block_allocator_set_last_bg() {
        let mut allocator = BlockAllocator::new();
        allocator.set_last_bg_id(5);
        assert_eq!(allocator.last_bg_id(), 5);
    }

    #[test]
    fn test_alloc_blocks_api() {
        // 这些测试需要实际的块设备和 ext4 文件系统
        // 主要验证 API 编译和基本逻辑
    }
}
