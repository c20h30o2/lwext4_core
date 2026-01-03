//! 目录校验和功能
//!
//! 对应 lwext4 的目录校验和相关功能

use crate::{
    block::BlockDevice,
    consts::*,
    fs::InodeRef,
    superblock::Superblock,
    types::ext4_dir_entry_tail,
};

/// 获取目录块的尾部（校验和结构）
///
/// 对应 lwext4 的 `ext4_dir_get_tail()`
///
/// # 参数
///
/// * `dirent_block` - 目录块数据
/// * `block_size` - 块大小
///
/// # 返回
///
/// 如果找到有效的校验和尾部则返回 Some，否则返回 None
///
/// # 安全性
///
/// 此函数对 packed 结构进行指针操作，需要确保：
/// - 输入块数据长度至少为 block_size
/// - block_size >= size_of::<ext4_dir_entry_tail>()
pub fn get_tail(dirent_block: &[u8], block_size: usize) -> Option<&ext4_dir_entry_tail> {
    // 检查块大小是否足够容纳尾部结构
    if block_size < core::mem::size_of::<ext4_dir_entry_tail>() {
        return None;
    }

    // 检查输入块数据是否足够长
    if dirent_block.len() < block_size {
        return None;
    }

    // 计算尾部结构的偏移量
    let tail_offset = block_size - core::mem::size_of::<ext4_dir_entry_tail>();

    // 获取尾部数据的切片
    let tail_bytes = &dirent_block[tail_offset..block_size];

    // 安全地将字节转换为结构体引用
    // 注意：ext4_dir_entry_tail 是 packed 结构，需要小心处理
    if tail_bytes.len() < core::mem::size_of::<ext4_dir_entry_tail>() {
        return None;
    }

    // 使用 unsafe 从字节切片创建结构体引用
    let tail = unsafe {
        &*(tail_bytes.as_ptr() as *const ext4_dir_entry_tail)
    };

    // 校验保留字段
    if tail.reserved_zero1 != 0 || tail.reserved_zero2 != 0 {
        return None;
    }

    // 校验记录长度
    if tail.rec_len() != core::mem::size_of::<ext4_dir_entry_tail>() as u16 {
        return None;
    }

    // 校验文件类型字段
    if tail.reserved_ft != EXT4_DIRENTRY_DIR_CSUM {
        return None;
    }

    Some(tail)
}

/// 获取目录块的可变尾部（校验和结构）
///
/// 对应 lwext4 的 `ext4_dir_get_tail()`
///
/// # 参数
///
/// * `dirent_block` - 目录块数据（可变）
/// * `block_size` - 块大小
///
/// # 返回
///
/// 如果找到有效的校验和尾部则返回 Some，否则返回 None
pub fn get_tail_mut(dirent_block: &mut [u8], block_size: usize) -> Option<&mut ext4_dir_entry_tail> {
    // 检查块大小是否足够容纳尾部结构
    if block_size < core::mem::size_of::<ext4_dir_entry_tail>() {
        return None;
    }

    // 检查输入块数据是否足够长
    if dirent_block.len() < block_size {
        return None;
    }

    // 计算尾部结构的偏移量
    let tail_offset = block_size - core::mem::size_of::<ext4_dir_entry_tail>();

    // 获取尾部数据的切片
    let tail_bytes = &mut dirent_block[tail_offset..block_size];

    // 安全地将字节转换为结构体引用
    if tail_bytes.len() < core::mem::size_of::<ext4_dir_entry_tail>() {
        return None;
    }

    // 使用 unsafe 从字节切片创建可变结构体引用
    let tail = unsafe {
        &mut *(tail_bytes.as_mut_ptr() as *mut ext4_dir_entry_tail)
    };

    // 校验保留字段
    if tail.reserved_zero1 != 0 || tail.reserved_zero2 != 0 {
        return None;
    }

    // 校验记录长度
    if tail.rec_len() != core::mem::size_of::<ext4_dir_entry_tail>() as u16 {
        return None;
    }

    // 校验文件类型字段
    if tail.reserved_ft != EXT4_DIRENTRY_DIR_CSUM {
        return None;
    }

    Some(tail)
}

/// 计算目录块的 CRC32 校验和
///
/// 对应 lwext4 的 `ext4_dir_csum()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode_ref` - inode 引用
/// * `dirent` - 目录块数据（不含尾部）
///
/// # 返回
///
/// CRC32 校验和值
#[cfg(feature = "metadata-csum")]
pub fn calculate_csum<D: BlockDevice>(sb: &Superblock, inode_ref: &InodeRef<D>, dirent: &[u8]) -> u32 {
    if !sb.has_ro_compat_feature(EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        return 0;
    }

    const EXT4_CRC32_INIT: u32 = 0xFFFFFFFF;

    // 1. 先计算 UUID 的校验和
    let mut csum = crate::crc::crc32c_append(EXT4_CRC32_INIT, sb.uuid());

    // 2. 然后计算 inode 号的校验和
    let ino_index = inode_ref.index().to_le_bytes();
    csum = crate::crc::crc32c_append(csum, &ino_index);

    // 3. 然后计算 inode generation 的校验和
    let ino_gen = inode_ref.generation().to_le_bytes();
    csum = crate::crc::crc32c_append(csum, &ino_gen);

    // 4. 最后计算目录项数据的校验和（不包含尾部）
    csum = crate::crc::crc32c_append(csum, dirent);

    csum
}

/// 占位实现：当未启用 metadata-csum 功能时
#[cfg(not(feature = "metadata-csum"))]
pub fn calculate_csum<D: BlockDevice>(_sb: &Superblock, _inode_ref: &InodeRef<D>, _dirent: &[u8]) -> u32 {
    0
}

/// 验证目录块的校验和
///
/// 对应 lwext4 的 `ext4_dir_csum_verify()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode_ref` - inode 引用
/// * `dirent_block` - 完整的目录块数据（包含尾部）
/// * `block_size` - 块大小
///
/// # 返回
///
/// 校验和是否正确
#[cfg(feature = "metadata-csum")]
pub fn verify_csum<D: BlockDevice>(
    sb: &Superblock,
    inode_ref: &InodeRef<D>,
    dirent_block: &[u8],
    block_size: usize,
) -> bool {
    // 只有当文件系统支持 metadata_csum 特性时才计算校验和
    if !sb.has_ro_compat_feature(EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        return true;
    }

    // 获取尾部结构
    let tail = match get_tail(dirent_block, block_size) {
        Some(t) => t,
        None => {
            // 没有空间容纳校验和
            return false;
        }
    };

    // 计算校验和（不包含尾部）
    let tail_offset = block_size - core::mem::size_of::<ext4_dir_entry_tail>();
    let dirent_data = &dirent_block[..tail_offset];
    let csum = calculate_csum(sb, inode_ref, dirent_data);

    // 比较校验和
    tail.checksum() == csum
}

/// 占位实现：当未启用 metadata-csum 功能时，总是返回 true
#[cfg(not(feature = "metadata-csum"))]
pub fn verify_csum<D: BlockDevice>(
    _sb: &Superblock,
    _inode_ref: &InodeRef<D>,
    _dirent_block: &[u8],
    _block_size: usize,
) -> bool {
    true
}

/// 初始化目录项尾部
///
/// 对应 lwext4 的 `ext4_dir_init_entry_tail()`
///
/// # 参数
///
/// * `tail` - 要初始化的尾部结构
pub fn init_entry_tail(tail: &mut ext4_dir_entry_tail) {
    tail.reserved_zero1 = 0;
    tail.reserved_zero2 = 0;
    tail.set_rec_len(core::mem::size_of::<ext4_dir_entry_tail>() as u16);
    tail.reserved_ft = EXT4_DIRENTRY_DIR_CSUM;
    tail.set_checksum(0);
}

/// 设置目录块的校验和
///
/// 对应 lwext4 的 `ext4_dir_set_csum()`
///
/// # 参数
///
/// * `sb` - superblock 引用
/// * `inode_ref` - inode 引用
/// * `dirent_block` - 完整的目录块数据（可变，包含尾部）
/// * `block_size` - 块大小
pub fn set_csum<D: BlockDevice>(
    sb: &Superblock,
    inode_ref: &InodeRef<D>,
    dirent_block: &mut [u8],
    block_size: usize,
) {
    // 只有当文件系统支持 metadata_csum 特性时才计算校验和
    if !sb.has_ro_compat_feature(EXT4_FEATURE_RO_COMPAT_METADATA_CSUM) {
        return;
    }

    // 获取可变尾部结构
    let tail_offset = block_size - core::mem::size_of::<ext4_dir_entry_tail>();

    // 先计算校验和（不包含尾部）
    let dirent_data = &dirent_block[..tail_offset];
    let csum = calculate_csum(sb, inode_ref, dirent_data);

    // 然后获取可变尾部并设置校验和
    if let Some(tail) = get_tail_mut(dirent_block, block_size) {
        tail.set_checksum(csum);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_init_entry_tail() {
        let mut tail = ext4_dir_entry_tail::default();
        init_entry_tail(&mut tail);

        // 使用原始指针访问 packed 结构的字段，避免创建引用
        unsafe {
            let ptr = &tail as *const ext4_dir_entry_tail;
            assert_eq!(core::ptr::addr_of!((*ptr).reserved_zero1).read_unaligned(), 0);
            assert_eq!(core::ptr::addr_of!((*ptr).reserved_zero2).read_unaligned(), 0);
            assert_eq!(tail.rec_len(), core::mem::size_of::<ext4_dir_entry_tail>() as u16);
            assert_eq!(core::ptr::addr_of!((*ptr).reserved_ft).read_unaligned(), EXT4_DIRENTRY_DIR_CSUM);
            assert_eq!(tail.checksum(), 0);
        }
    }

    #[test]
    fn test_get_tail_invalid_size() {
        let block = vec![0u8; 512];
        let tail = get_tail(&block, 512);

        // 应该找不到有效的尾部（因为没有初始化正确的标志）
        assert!(tail.is_none());
    }

    #[test]
    fn test_get_tail_with_valid_tail() {
        let block_size = 1024;
        let mut block = vec![0u8; block_size];

        // 在块尾部初始化一个有效的尾部结构
        let tail_offset = block_size - core::mem::size_of::<ext4_dir_entry_tail>();
        let tail_bytes = &mut block[tail_offset..];

        let tail = unsafe {
            &mut *(tail_bytes.as_mut_ptr() as *mut ext4_dir_entry_tail)
        };
        init_entry_tail(tail);

        // 现在应该能找到有效的尾部
        let found_tail = get_tail(&block, block_size);
        assert!(found_tail.is_some());

        let found = found_tail.unwrap();
        assert_eq!(found.rec_len(), core::mem::size_of::<ext4_dir_entry_tail>() as u16);
        unsafe {
            let ptr = found as *const ext4_dir_entry_tail;
            assert_eq!(core::ptr::addr_of!((*ptr).reserved_ft).read_unaligned(), EXT4_DIRENTRY_DIR_CSUM);
        }
    }
}
