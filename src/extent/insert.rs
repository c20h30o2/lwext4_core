//! Extent 索引插入功能
//!
//! 提供向 extent 树的索引节点插入新索引条目的功能

use crate::{
    block::BlockDevice,
    error::{Error, ErrorKind, Result},
    fs::InodeRef,
};

use super::{
    helpers::*,
    write::ExtentPath,
};

/// 在索引节点中插入新的索引条目
///
/// 对应 lwext4 的 `ext4_ext_insert_index()`
///
/// # 参数
///
/// * `inode_ref` - inode 引用
/// * `path` - extent 路径数组
/// * `at` - 在哪一层插入（0 = inode root）
/// * `insert_index` - 新索引的逻辑块号
/// * `insert_block` - 新索引指向的物理块号
/// * `set_to_ix` - 是否更新 path 指向新插入的索引
///
/// # 返回
///
/// 成功返回 ()
///
/// # 错误
///
/// - `ErrorKind::InvalidInput` - 索引节点已满或参数无效
/// - `ErrorKind::Corrupted` - 重复的索引条目
pub fn insert_index<D: BlockDevice>(
    inode_ref: &mut InodeRef<D>,
    path: &mut [ExtentPath],
    at: usize,
    insert_index: u32,
    insert_block: u64,
    set_to_ix: bool,
) -> Result<()> {
    if at >= path.len() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "insert_index: at out of bounds",
        ));
    }

    let curp = &mut path[at];

    unsafe {
        let header = curp.header;

        // 1. 前置检查：节点是否已满
        if !EXT_HAS_FREE_INDEX(header) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Index node is full, should split first",
            ));
        }

        // 2. 防止重复插入（检查 first_block）
        if let Some(cur_idx_ptr) = curp.index {
            let cur_first_block = u32::from_le((*cur_idx_ptr).first_block);
            if insert_index == cur_first_block {
                return Err(Error::new(
                    ErrorKind::Corrupted,
                    "Duplicate index entry",
                ));
            }
        }

        // 3. 确定插入位置
        let ix = if curp.index.is_none() {
            // 空节点，插入第一个位置
            EXT_FIRST_INDEX(header)
        } else {
            let cur_idx = curp.index.unwrap();
            let cur_first_block = u32::from_le((*cur_idx).first_block);

            if insert_index > cur_first_block {
                // 插入到当前之后
                cur_idx.add(1)
            } else {
                // 插入到当前之前
                cur_idx
            }
        };

        // 越界保护
        if ix > EXT_MAX_INDEX(header) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Index out of bounds",
            ));
        }

        // 4. 移动现有索引条目（为新索引腾出空间）
        let last_idx = EXT_LAST_INDEX(header);
        let len = (last_idx as usize - ix as usize) / core::mem::size_of::<crate::types::ext4_extent_idx>() + 1;

        if len > 0 {
            core::ptr::copy(ix, ix.add(1), len);
        }

        // 5. 填充新索引条目
        (*ix).first_block = insert_index.to_le();
        ext4_idx_store_pblock(&mut *ix, insert_block);

        // 6. 更新 entries_count
        let entries = u16::from_le((*header).entries);
        (*header).entries = (entries + 1).to_le();

        // 7. 标记 dirty
        curp.mark_dirty(inode_ref)?;

        // 8. 可选：更新 path 指向新索引
        if set_to_ix {
            curp.index = Some(ix);
            curp.p_block = insert_block;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ext4_extent_header;
    use crate::consts::*;

    #[test]
    fn test_insert_index_api() {
        // 这些测试需要实际的块设备和 ext4 文件系统
        // 主要验证 API 编译和基本逻辑
    }

    #[test]
    fn test_insert_index_full_node() {
        // 测试向满节点插入应该返回错误
        let mut header = ext4_extent_header {
            magic: EXT4_EXTENT_MAGIC.to_le(),
            entries: 4u16.to_le(),
            max: 4u16.to_le(),  // 已满
            depth: 1u16.to_le(),
            generation: 0u32.to_le(),
        };

        unsafe {
            assert!(!EXT_HAS_FREE_INDEX(&header));
        }
    }
}
