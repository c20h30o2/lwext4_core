//! JBD2 (Journal Block Device version 2) 磁盘格式定义
//!
//! 这个模块定义了ext4 journal的所有磁盘格式结构。
//!
//! # 重要说明
//!
//! - **所有字段都是大端序（big-endian）**
//! - 需要使用 `to_be32()` / `from_be32()` 等函数进行字节序转换
//! - 使用 `#[repr(C, packed)]` 确保内存布局与磁盘格式一致
//!
//! # 对应关系
//!
//! 对应 lwext4 的 `ext4_types.h` 中的 JBD2 结构定义（第620-783行）

/// UUID 大小（128位）
pub const UUID_SIZE: usize = 16;

/// Journal 最大用户数
pub const JBD_USERS_MAX: usize = 48;

/// Journal 用户区域大小
pub const JBD_USERS_SIZE: usize = UUID_SIZE * JBD_USERS_MAX;

/// CRC32 校验和字节数
pub const JBD_CHECKSUM_BYTES: usize = 8;

// =============================================================================
// Block Header and Types
// =============================================================================

/// JBD2 块头（所有描述符块的标准头部）
///
/// 对应 lwext4 的 `struct jbd_bhdr`
///
/// # 字节序
///
/// 所有字段都是大端序（big-endian）
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct jbd_bhdr {
    /// Magic number (0xC03B3998)
    pub magic: u32,
    /// Block type (descriptor, commit, revoke, superblock)
    pub blocktype: u32,
    /// Transaction sequence number
    pub sequence: u32,
}

/// JBD2 magic number
pub const JBD_MAGIC_NUMBER: u32 = 0xC03B3998;

/// Block types
pub const JBD_DESCRIPTOR_BLOCK: u32 = 1;
pub const JBD_COMMIT_BLOCK: u32 = 2;
pub const JBD_SUPERBLOCK_V1: u32 = 3;
pub const JBD_SUPERBLOCK_V2: u32 = 4;
pub const JBD_REVOKE_BLOCK: u32 = 5;

// =============================================================================
// Checksum Types
// =============================================================================

/// Checksum types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JbdChecksumType {
    Crc32 = 1,
    Md5 = 2,
    Sha1 = 3,
    Crc32c = 4,
}

/// CRC32 checksum size (bytes)
pub const JBD_CRC32_CHKSUM_SIZE: usize = 4;

// =============================================================================
// Commit Block
// =============================================================================

/// Commit 块头（存储事务校验和）
///
/// 对应 lwext4 的 `struct jbd_commit_header`
///
/// # Checksum Versions
///
/// - **v1** (FEATURE_COMPAT_CHECKSUM): chksum[] 存储描述符和数据块校验和
/// - **v2** (FEATURE_INCOMPAT_CSUM_V2): chksum[0] 存储 crc32c(uuid+commit_block)
/// - **v3** (FEATURE_INCOMPAT_CSUM_V3): 使用 journal_block_tag3 存储完整32位校验和
///
/// 这三个版本互斥。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct jbd_commit_header {
    /// Block header
    pub header: jbd_bhdr,
    /// Checksum type
    pub chksum_type: u8,
    /// Checksum size
    pub chksum_size: u8,
    /// Padding
    pub padding: [u8; 2],
    /// Checksum array (8 u32 = 32 bytes)
    pub chksum: [u32; JBD_CHECKSUM_BYTES],
    /// Commit timestamp (seconds since epoch)
    pub commit_sec: u64,
    /// Commit timestamp (nanoseconds)
    pub commit_nsec: u32,
}

// =============================================================================
// Block Tags (Descriptor Block)
// =============================================================================

/// Block tag v3（64位块号，完整校验和）
///
/// 对应 lwext4 的 `struct jbd_block_tag3`
///
/// 用于 FEATURE_INCOMPAT_CSUM_V3 特性
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct jbd_block_tag3 {
    /// On-disk block number (low 32 bits)
    pub blocknr: u32,
    /// Flags (see JBD_FLAG_*)
    pub flags: u32,
    /// On-disk block number (high 32 bits)
    pub blocknr_high: u32,
    /// Full 32-bit checksum: crc32c(uuid+seq+block)
    pub checksum: u32,
}

/// Block tag（标准版本，16位截断校验和）
///
/// 对应 lwext4 的 `struct jbd_block_tag`
///
/// 用于标准JBD2和CSUM_V2
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct jbd_block_tag {
    /// On-disk block number (low 32 bits)
    pub blocknr: u32,
    /// Truncated crc32c checksum (16 bits)
    pub checksum: u16,
    /// Flags (see JBD_FLAG_*)
    pub flags: u16,
    /// On-disk block number (high 32 bits, for 64-bit mode)
    pub blocknr_high: u32,
}

// Tag flags
/// On-disk block is escaped (starts with JBD magic)
pub const JBD_FLAG_ESCAPE: u16 = 1;
/// Block has same UUID as previous
pub const JBD_FLAG_SAME_UUID: u16 = 2;
/// Block deleted by this transaction
pub const JBD_FLAG_DELETED: u16 = 4;
/// Last tag in this descriptor block
pub const JBD_FLAG_LAST_TAG: u16 = 8;

// =============================================================================
// Block Tail (for checksumming)
// =============================================================================

/// Descriptor 块尾部（用于校验和）
///
/// 对应 lwext4 的 `struct jbd_block_tail`
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct jbd_block_tail {
    /// Checksum of the descriptor block
    pub checksum: u32,
}

// =============================================================================
// Revoke Block
// =============================================================================

/// Revoke 块头（描述要从日志中撤销的块）
///
/// 对应 lwext4 的 `struct jbd_revoke_header`
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct jbd_revoke_header {
    /// Block header
    pub header: jbd_bhdr,
    /// Count of bytes used in this block
    pub count: u32,
}

/// Revoke 块尾部（用于校验和）
///
/// 对应 lwext4 的 `struct jbd_revoke_tail`
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct jbd_revoke_tail {
    /// Checksum of the revoke block
    pub checksum: u32,
}

// =============================================================================
// Journal Superblock
// =============================================================================

/// Journal 超级块
///
/// 对应 lwext4 的 `struct jbd_sb`
///
/// # 字节序
///
/// **所有字段都是大端序（big-endian）**
///
/// # 布局
///
/// ```text
/// Offset  Size  Field
/// 0x0000  12    header (jbd_bhdr)
/// 0x000C  4     blocksize
/// 0x0010  4     maxlen
/// 0x0014  4     first
/// 0x0018  4     sequence
/// 0x001C  4     start
/// 0x0020  4     error_val
/// 0x0024  4     feature_compat
/// 0x0028  4     feature_incompat
/// 0x002C  4     feature_ro_compat
/// 0x0030  16    uuid
/// 0x0040  4     nr_users
/// 0x0044  4     dynsuper
/// 0x0048  4     max_transaction
/// 0x004C  4     max_trandata
/// 0x0050  1     checksum_type
/// 0x0051  3     padding2
/// 0x0054  168   padding (42 * 4)
/// 0x00FC  4     checksum
/// 0x0100  768   users (48 * 16)
/// 0x0400  END   (total 1024 bytes)
/// ```
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct jbd_sb {
    /* 0x0000 */
    /// Block header
    pub header: jbd_bhdr,

    /* 0x000C */
    /// Journal device block size
    pub blocksize: u32,
    /// Total blocks in journal file
    pub maxlen: u32,
    /// First block of log information
    pub first: u32,

    /* 0x0018 */
    /// First commit ID expected in log
    pub sequence: u32,
    /// Block number of start of log
    pub start: u32,

    /* 0x0020 */
    /// Error value (as set by journal_abort)
    pub error_val: i32,

    /* 0x0024 */
    /// Compatible feature set
    pub feature_compat: u32,
    /// Incompatible feature set
    pub feature_incompat: u32,
    /// Read-only compatible feature set
    pub feature_ro_compat: u32,

    /* 0x0030 */
    /// 128-bit UUID for journal
    pub uuid: [u8; UUID_SIZE],

    /* 0x0040 */
    /// Number of filesystems sharing log
    pub nr_users: u32,
    /// Block number of dynamic superblock copy
    pub dynsuper: u32,

    /* 0x0048 */
    /// Limit of journal blocks per transaction
    pub max_transaction: u32,
    /// Limit of data blocks per transaction
    pub max_trandata: u32,

    /* 0x0050 */
    /// Checksum type
    pub checksum_type: u8,
    /// Padding
    pub padding2: [u8; 3],
    /// Reserved padding (42 u32s = 168 bytes)
    pub padding: [u32; 42],
    /// CRC32C checksum of superblock
    pub checksum: u32,

    /* 0x0100 */
    /// IDs of all filesystems sharing the log
    pub users: [u8; JBD_USERS_SIZE],

    /* 0x0400 - Total size: 1024 bytes */
}

/// Superblock size (must be 1024 bytes)
pub const JBD_SUPERBLOCK_SIZE: usize = core::mem::size_of::<jbd_sb>();

// Compile-time assertion: ensure superblock is exactly 1024 bytes
const _: () = assert!(JBD_SUPERBLOCK_SIZE == 1024, "jbd_sb must be exactly 1024 bytes");

// =============================================================================
// Feature Flags
// =============================================================================

/// Feature: Checksum v1
pub const JBD_FEATURE_COMPAT_CHECKSUM: u32 = 0x00000001;

/// Feature: Revoke support
pub const JBD_FEATURE_INCOMPAT_REVOKE: u32 = 0x00000001;
/// Feature: 64-bit block numbers
pub const JBD_FEATURE_INCOMPAT_64BIT: u32 = 0x00000002;
/// Feature: Async commit
pub const JBD_FEATURE_INCOMPAT_ASYNC_COMMIT: u32 = 0x00000004;
/// Feature: Checksum v2 (crc32c)
pub const JBD_FEATURE_INCOMPAT_CSUM_V2: u32 = 0x00000008;
/// Feature: Checksum v3 (full 32-bit checksum in tags)
pub const JBD_FEATURE_INCOMPAT_CSUM_V3: u32 = 0x00000010;

/// Known compatible features
pub const JBD_KNOWN_COMPAT_FEATURES: u32 = 0;
/// Known read-only compatible features
pub const JBD_KNOWN_ROCOMPAT_FEATURES: u32 = 0;
/// Known incompatible features
pub const JBD_KNOWN_INCOMPAT_FEATURES: u32 = JBD_FEATURE_INCOMPAT_REVOKE
    | JBD_FEATURE_INCOMPAT_ASYNC_COMMIT
    | JBD_FEATURE_INCOMPAT_64BIT
    | JBD_FEATURE_INCOMPAT_CSUM_V2
    | JBD_FEATURE_INCOMPAT_CSUM_V3;

// =============================================================================
// Helper Functions
// =============================================================================

impl jbd_sb {
    /// Verify journal superblock magic number
    pub fn is_valid(&self) -> bool {
        u32::from_be(self.header.magic) == JBD_MAGIC_NUMBER
    }

    /// Check if journal has a specific compatible feature
    pub fn has_compat_feature(&self, feature: u32) -> bool {
        u32::from_be(self.header.blocktype) >= 2 && (u32::from_be(self.feature_compat) & feature) != 0
    }

    /// Check if journal has a specific incompatible feature
    pub fn has_incompat_feature(&self, feature: u32) -> bool {
        u32::from_be(self.header.blocktype) >= 2
            && (u32::from_be(self.feature_incompat) & feature) != 0
    }

    /// Check if journal has a specific read-only compatible feature
    pub fn has_ro_compat_feature(&self, feature: u32) -> bool {
        u32::from_be(self.header.blocktype) >= 2
            && (u32::from_be(self.feature_ro_compat) & feature) != 0
    }

    /// Get checksum version (0, 2, or 3)
    pub fn checksum_version(&self) -> u8 {
        if self.has_incompat_feature(JBD_FEATURE_INCOMPAT_CSUM_V3) {
            3
        } else if self.has_incompat_feature(JBD_FEATURE_INCOMPAT_CSUM_V2) {
            2
        } else {
            0
        }
    }

    /// Check if journal supports 64-bit block numbers
    pub fn is_64bit(&self) -> bool {
        self.has_incompat_feature(JBD_FEATURE_INCOMPAT_64BIT)
    }
}

impl Default for jbd_sb {
    fn default() -> Self {
        Self {
            header: jbd_bhdr {
                magic: JBD_MAGIC_NUMBER.to_be(),
                blocktype: JBD_SUPERBLOCK_V2.to_be(),
                sequence: 0,
            },
            blocksize: 4096u32.to_be(),
            maxlen: 0,
            first: 0,
            sequence: 0,
            start: 0,
            error_val: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            uuid: [0; UUID_SIZE],
            nr_users: 0,
            dynsuper: 0,
            max_transaction: 0,
            max_trandata: 0,
            checksum_type: 0,
            padding2: [0; 3],
            padding: [0; 42],
            checksum: 0,
            users: [0; JBD_USERS_SIZE],
        }
    }
}

impl jbd_bhdr {
    /// Create a new block header
    pub fn new(blocktype: u32, sequence: u32) -> Self {
        Self {
            magic: JBD_MAGIC_NUMBER.to_be(),
            blocktype: blocktype.to_be(),
            sequence: sequence.to_be(),
        }
    }

    /// Verify magic number
    pub fn verify_magic(&self) -> bool {
        u32::from_be(self.magic) == JBD_MAGIC_NUMBER
    }

    /// Get block type (native endian)
    pub fn get_blocktype(&self) -> u32 {
        u32::from_be(self.blocktype)
    }

    /// Get sequence number (native endian)
    pub fn get_sequence(&self) -> u32 {
        u32::from_be(self.sequence)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_superblock_size() {
        assert_eq!(
            core::mem::size_of::<jbd_sb>(),
            1024,
            "jbd_sb must be exactly 1024 bytes"
        );
    }

    #[test]
    fn test_bhdr_size() {
        assert_eq!(
            core::mem::size_of::<jbd_bhdr>(),
            12,
            "jbd_bhdr must be 12 bytes"
        );
    }

    #[test]
    fn test_commit_header_size() {
        // header(12) + chksum_type(1) + chksum_size(1) + padding(2) + chksum(32) + commit_sec(8) + commit_nsec(4) = 60
        assert_eq!(
            core::mem::size_of::<jbd_commit_header>(),
            60,
            "jbd_commit_header size mismatch"
        );
    }

    #[test]
    fn test_block_tag3_size() {
        // blocknr(4) + flags(4) + blocknr_high(4) + checksum(4) = 16
        assert_eq!(
            core::mem::size_of::<jbd_block_tag3>(),
            16,
            "jbd_block_tag3 must be 16 bytes"
        );
    }

    #[test]
    fn test_block_tag_size() {
        // blocknr(4) + checksum(2) + flags(2) + blocknr_high(4) = 12
        assert_eq!(
            core::mem::size_of::<jbd_block_tag>(),
            12,
            "jbd_block_tag must be 12 bytes"
        );
    }

    #[test]
    fn test_bhdr_endian() {
        let header = jbd_bhdr::new(JBD_DESCRIPTOR_BLOCK, 100);
        assert!(header.verify_magic());
        assert_eq!(header.get_blocktype(), JBD_DESCRIPTOR_BLOCK);
        assert_eq!(header.get_sequence(), 100);
    }
}
