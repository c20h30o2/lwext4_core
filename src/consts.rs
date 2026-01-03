//! ext4 文件系统常量定义
//!
//! 这个模块包含了 ext4 文件系统的所有常量定义，包括：
//! - 磁盘布局相关常量
//! - 文件类型和权限位
//! - 特性标志
//! - 错误码

//=============================================================================
// 基础常量
//=============================================================================

/// 默认物理块大小（扇区大小，512 字节）
pub const EXT4_DEFAULT_SECTOR_SIZE: u32 = 512;

/// 默认逻辑块大小（4096 字节）
pub const EXT4_DEFAULT_BLOCK_SIZE: u32 = 4096;

/// 最小块大小（1024 字节）
pub const EXT4_MIN_BLOCK_SIZE: u32 = 1024;

/// 最大块大小（65536 字节）
pub const EXT4_MAX_BLOCK_SIZE: u32 = 65536;

//=============================================================================
// Superblock 相关
//=============================================================================

/// Superblock 在设备上的字节偏移
pub const EXT4_SUPERBLOCK_OFFSET: u64 = 1024;

/// Superblock 大小（字节）
pub const EXT4_SUPERBLOCK_SIZE: usize = 1024;

/// ext4 魔数 (0xEF53)
pub const EXT4_SUPERBLOCK_MAGIC: u16 = 0xEF53;

/// Extent 树魔数 (0xF30A)
pub const EXT4_EXTENT_MAGIC: u16 = 0xF30A;

/// Root inode 编号
pub const EXT4_ROOT_INODE: u32 = 2;

/// 块组描述符大小（传统）
pub const EXT4_GROUP_DESC_SIZE: usize = 32;

/// 块组描述符大小（64位）
pub const EXT4_GROUP_DESC_SIZE_64: usize = 64;

/// 块组描述符最小大小
pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: usize = 32;

/// 块组描述符最大大小
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: usize = 1024;

/// Superblock 状态：有效/已挂载
pub const EXT4_SUPER_STATE_VALID: u16 = 0x0001;

/// Superblock 状态：有错误
pub const EXT4_SUPER_STATE_ERROR: u16 = 0x0002;

/// Superblock 状态：孤儿恢复中
pub const EXT4_SUPER_STATE_ORPHAN: u16 = 0x0004;

/// 校验和类型：CRC32C
pub const EXT4_CHECKSUM_CRC32C: u8 = 1;

/// CRC32C 初始值
pub const EXT4_CRC32_INIT: u32 = !0u32; // 0xFFFFFFFF

/// Superblock flags: Signed directory hash in use
pub const EXT4_SUPERBLOCK_FLAGS_SIGNED_HASH: u32 = 0x0001;

/// Superblock flags: Unsigned directory hash in use
pub const EXT4_SUPERBLOCK_FLAGS_UNSIGNED_HASH: u32 = 0x0002;

/// Superblock flags: Test development code
pub const EXT4_SUPERBLOCK_FLAGS_TEST_FILESYS: u32 = 0x0004;

//=============================================================================
// Inode 相关
//=============================================================================

/// Inode 中的块指针总数（15个）
/// - 12个直接块
/// - 1个一级间接块
/// - 1个二级间接块
/// - 1个三级间接块
pub const EXT4_INODE_BLOCKS: usize = 15;

/// 直接块指针数量
pub const EXT4_INODE_DIRECT_BLOCKS: usize = 12;

/// 一级间接块索引
pub const EXT4_INODE_INDIRECT_BLOCK: usize = 12;

/// 二级间接块索引
pub const EXT4_INODE_DOUBLE_INDIRECT_BLOCK: usize = 13;

/// 三级间接块索引
pub const EXT4_INODE_TRIPLE_INDIRECT_BLOCK: usize = 14;

/// 默认 inode 大小
pub const EXT4_DEFAULT_INODE_SIZE: u16 = 128;

/// 大 inode 的默认大小（带扩展属性）
pub const EXT4_LARGE_INODE_SIZE: u16 = 256;

/// 旧的 inode 大小（不含扩展字段）
pub const EXT4_GOOD_OLD_INODE_SIZE: usize = 128;

//=============================================================================
// Superblock OS 相关
//=============================================================================

/// Linux 操作系统
pub const EXT4_SUPERBLOCK_OS_LINUX: u32 = 0;

/// Hurd 操作系统
pub const EXT4_SUPERBLOCK_OS_HURD: u32 = 1;

//=============================================================================
// Inode 模式位（文件类型和权限）
//=============================================================================

/// 文件类型掩码
pub const EXT4_INODE_MODE_TYPE_MASK: u16 = 0xF000;

/// FIFO
pub const EXT4_INODE_MODE_FIFO: u16 = 0x1000;

/// 字符设备
pub const EXT4_INODE_MODE_CHARDEV: u16 = 0x2000;

/// 目录
pub const EXT4_INODE_MODE_DIRECTORY: u16 = 0x4000;

/// 块设备
pub const EXT4_INODE_MODE_BLOCKDEV: u16 = 0x6000;

/// 普通文件
pub const EXT4_INODE_MODE_FILE: u16 = 0x8000;

/// 符号链接
pub const EXT4_INODE_MODE_SOFTLINK: u16 = 0xA000;

/// Socket
pub const EXT4_INODE_MODE_SOCKET: u16 = 0xC000;

/// 权限位掩码
pub const EXT4_INODE_MODE_PERM_MASK: u16 = 0x0FFF;

/// 用户读权限
pub const EXT4_INODE_MODE_USER_READ: u16 = 0x0100;

/// 用户写权限
pub const EXT4_INODE_MODE_USER_WRITE: u16 = 0x0080;

/// 用户执行权限
pub const EXT4_INODE_MODE_USER_EXEC: u16 = 0x0040;

/// 组读权限
pub const EXT4_INODE_MODE_GROUP_READ: u16 = 0x0020;

/// 组写权限
pub const EXT4_INODE_MODE_GROUP_WRITE: u16 = 0x0010;

/// 组执行权限
pub const EXT4_INODE_MODE_GROUP_EXEC: u16 = 0x0008;

/// 其他用户读权限
pub const EXT4_INODE_MODE_OTHER_READ: u16 = 0x0004;

/// 其他用户写权限
pub const EXT4_INODE_MODE_OTHER_WRITE: u16 = 0x0002;

/// 其他用户执行权限
pub const EXT4_INODE_MODE_OTHER_EXEC: u16 = 0x0001;

//=============================================================================
// Inode 标志
//=============================================================================

/// 使用 extent 树存储文件数据
pub const EXT4_INODE_FLAG_EXTENTS: u32 = 0x00080000;

/// 大文件（>= 2GB）
pub const EXT4_INODE_FLAG_HUGE_FILE: u32 = 0x00040000;

/// 目录使用哈希树索引
pub const EXT4_INODE_FLAG_INDEX: u32 = 0x00001000;

/// 不可变文件
pub const EXT4_INODE_FLAG_IMMUTABLE: u32 = 0x00000010;

/// 仅追加
pub const EXT4_INODE_FLAG_APPEND: u32 = 0x00000020;

//=============================================================================
// 目录项类型
//=============================================================================

/// 未知类型
pub const EXT4_DE_UNKNOWN: u8 = 0;

/// 普通文件
pub const EXT4_DE_REG_FILE: u8 = 1;

/// 目录
pub const EXT4_DE_DIR: u8 = 2;

/// 字符设备
pub const EXT4_DE_CHRDEV: u8 = 3;

/// 块设备
pub const EXT4_DE_BLKDEV: u8 = 4;

/// FIFO
pub const EXT4_DE_FIFO: u8 = 5;

/// Socket
pub const EXT4_DE_SOCK: u8 = 6;

/// 符号链接
pub const EXT4_DE_SYMLINK: u8 = 7;

/// 目录项最小长度
pub const EXT4_DIR_ENTRY_MIN_LEN: usize = 8;

/// 目录项对齐边界
pub const EXT4_DIR_ENTRY_ALIGN: usize = 4;

/// 目录校验和类型标志（用于目录项尾部）
pub const EXT4_DIRENTRY_DIR_CSUM: u8 = 0xDE;

/// 最大文件名长度
pub const EXT4_NAME_MAX: usize = 255;

//=============================================================================
// 特性标志（兼容性）
//=============================================================================

/// 兼容特性：目录预分配
pub const EXT4_FEATURE_COMPAT_DIR_PREALLOC: u32 = 0x0001;

/// 兼容特性：has journal
pub const EXT4_FEATURE_COMPAT_HAS_JOURNAL: u32 = 0x0004;

/// 兼容特性：resize inode
pub const EXT4_FEATURE_COMPAT_RESIZE_INODE: u32 = 0x0010;

/// 兼容特性：目录索引
pub const EXT4_FEATURE_COMPAT_DIR_INDEX: u32 = 0x0020;

/// 兼容特性：延迟 inode 表初始化
pub const EXT4_FEATURE_COMPAT_LAZY_BG: u32 = 0x0040;

/// 不兼容特性：压缩
pub const EXT4_FEATURE_INCOMPAT_COMPRESSION: u32 = 0x0001;

/// 不兼容特性：目录项包含文件类型
pub const EXT4_FEATURE_INCOMPAT_FILETYPE: u32 = 0x0002;

/// 不兼容特性：需要恢复
pub const EXT4_FEATURE_INCOMPAT_RECOVER: u32 = 0x0004;

/// 不兼容特性：日志设备
pub const EXT4_FEATURE_INCOMPAT_JOURNAL_DEV: u32 = 0x0008;

/// 不兼容特性：元数据块组
pub const EXT4_FEATURE_INCOMPAT_META_BG: u32 = 0x0010;

/// 不兼容特性：extent
pub const EXT4_FEATURE_INCOMPAT_EXTENTS: u32 = 0x0040;

/// 不兼容特性：64位
pub const EXT4_FEATURE_INCOMPAT_64BIT: u32 = 0x0080;

/// 不兼容特性：多挂载保护
pub const EXT4_FEATURE_INCOMPAT_MMP: u32 = 0x0100;

/// 不兼容特性：flex_bg
pub const EXT4_FEATURE_INCOMPAT_FLEX_BG: u32 = 0x0200;

/// 不兼容特性：大 extended attribute
pub const EXT4_FEATURE_INCOMPAT_EA_INODE: u32 = 0x0400;

/// 不兼容特性：目录数据内联
pub const EXT4_FEATURE_INCOMPAT_DIRDATA: u32 = 0x1000;

/// 不兼容特性：元数据校验和
pub const EXT4_FEATURE_INCOMPAT_CSUM_SEED: u32 = 0x2000;

/// 不兼容特性：大目录
pub const EXT4_FEATURE_INCOMPAT_LARGEDIR: u32 = 0x4000;

/// 不兼容特性：数据内联
pub const EXT4_FEATURE_INCOMPAT_INLINE_DATA: u32 = 0x8000;

/// 不兼容特性：加密
pub const EXT4_FEATURE_INCOMPAT_ENCRYPT: u32 = 0x10000;

/// 只读兼容特性：稀疏超级块
pub const EXT4_FEATURE_RO_COMPAT_SPARSE_SUPER: u32 = 0x0001;

/// 只读兼容特性：大文件
pub const EXT4_FEATURE_RO_COMPAT_LARGE_FILE: u32 = 0x0002;

/// 只读兼容特性：B树目录
pub const EXT4_FEATURE_RO_COMPAT_BTREE_DIR: u32 = 0x0004;

/// 只读兼容特性：巨文件
pub const EXT4_FEATURE_RO_COMPAT_HUGE_FILE: u32 = 0x0008;

/// 只读兼容特性：GDT校验和
pub const EXT4_FEATURE_RO_COMPAT_GDT_CSUM: u32 = 0x0010;

/// 只读兼容特性：大目录
pub const EXT4_FEATURE_RO_COMPAT_DIR_NLINK: u32 = 0x0020;

/// 只读兼容特性：大 inode
pub const EXT4_FEATURE_RO_COMPAT_EXTRA_ISIZE: u32 = 0x0040;

/// 只读兼容特性：快照
pub const EXT4_FEATURE_RO_COMPAT_HAS_SNAPSHOT: u32 = 0x0080;

/// 只读兼容特性：配额
pub const EXT4_FEATURE_RO_COMPAT_QUOTA: u32 = 0x0100;

/// 只读兼容特性：bigalloc
pub const EXT4_FEATURE_RO_COMPAT_BIGALLOC: u32 = 0x0200;

/// 只读兼容特性：元数据校验和
pub const EXT4_FEATURE_RO_COMPAT_METADATA_CSUM: u32 = 0x0400;

/// 只读兼容特性：只读
pub const EXT4_FEATURE_RO_COMPAT_READONLY: u32 = 0x1000;

/// 只读兼容特性：项目配额
pub const EXT4_FEATURE_RO_COMPAT_PROJECT: u32 = 0x2000;

//=============================================================================
// 缓存和性能相关
//=============================================================================

/// 块设备缓存大小（缓存的块数量）
pub const CONFIG_BLOCK_DEV_CACHE_SIZE: u32 = 8;

/// 最大缓存引用块数
pub const CONFIG_MAX_CACHE_REF_BLOCKS: u32 = 256;

//=============================================================================
// 错误码（与 POSIX errno 兼容）
//=============================================================================

/// 成功
pub const EOK: i32 = 0;

/// 没有此文件或目录
pub const ENOENT: i32 = 2;

/// I/O 错误
pub const EIO: i32 = 5;

/// 内存不足
pub const ENOMEM: i32 = 12;

/// 是一个目录
pub const EISDIR: i32 = 21;

/// 无效参数
pub const EINVAL: i32 = 22;

/// 设备上没有空间
pub const ENOSPC: i32 = 28;

/// 目录非空
pub const ENOTEMPTY: i32 = 39;

/// 不支持的操作
pub const ENOTSUP: i32 = 95;

//=============================================================================
// 限制
//=============================================================================

/// 最大路径长度
pub const EXT4_PATH_MAX: usize = 4096;

/// 最大符号链接深度
pub const EXT4_LINK_MAX: u32 = 65000;

/// 每个 inode 的最大 extent 数
pub const EXT4_EXTENT_MAX_DEPTH: u8 = 5;

//=============================================================================
// Extended Attributes (xattr) 常量
//=============================================================================

/// xattr 魔数
pub const EXT4_XATTR_MAGIC: u32 = 0xEA020000;

/// xattr 最大引用计数
pub const EXT4_XATTR_REFCOUNT_MAX: u32 = 1024;

/// xattr 对齐（4字节对齐）
pub const EXT4_XATTR_PAD_BITS: u32 = 2;
pub const EXT4_XATTR_PAD: u32 = 1 << EXT4_XATTR_PAD_BITS;
pub const EXT4_XATTR_ROUND: u32 = EXT4_XATTR_PAD - 1;

/// xattr 命名空间索引
pub const EXT4_XATTR_INDEX_USER: u8 = 1;
pub const EXT4_XATTR_INDEX_POSIX_ACL_ACCESS: u8 = 2;
pub const EXT4_XATTR_INDEX_POSIX_ACL_DEFAULT: u8 = 3;
pub const EXT4_XATTR_INDEX_TRUSTED: u8 = 4;
pub const EXT4_XATTR_INDEX_LUSTRE: u8 = 5;
pub const EXT4_XATTR_INDEX_SECURITY: u8 = 6;
pub const EXT4_XATTR_INDEX_SYSTEM: u8 = 7;
pub const EXT4_XATTR_INDEX_RICHACL: u8 = 8;
pub const EXT4_XATTR_INDEX_ENCRYPTION: u8 = 9;

/// 哈希计算相关
pub const NAME_HASH_SHIFT: u32 = 5;
pub const VALUE_HASH_SHIFT: u32 = 16;
pub const BLOCK_HASH_SHIFT: u32 = 16;
