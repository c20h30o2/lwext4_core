//! xattr 命名空间前缀处理
//!
//! 处理扩展属性的命名空间前缀（如 "user.", "system." 等）

use crate::consts::*;

/// 命名空间前缀表条目
struct XattrPrefix {
    prefix: &'static str,
    name_index: u8,
}

/// 命名空间前缀表
///
/// 对应 lwext4 的 prefix_tbl
static PREFIX_TABLE: &[XattrPrefix] = &[
    XattrPrefix {
        prefix: "user.",
        name_index: EXT4_XATTR_INDEX_USER,
    },
    XattrPrefix {
        prefix: "system.posix_acl_access",
        name_index: EXT4_XATTR_INDEX_POSIX_ACL_ACCESS,
    },
    XattrPrefix {
        prefix: "system.posix_acl_default",
        name_index: EXT4_XATTR_INDEX_POSIX_ACL_DEFAULT,
    },
    XattrPrefix {
        prefix: "trusted.",
        name_index: EXT4_XATTR_INDEX_TRUSTED,
    },
    XattrPrefix {
        prefix: "security.",
        name_index: EXT4_XATTR_INDEX_SECURITY,
    },
    XattrPrefix {
        prefix: "system.",
        name_index: EXT4_XATTR_INDEX_SYSTEM,
    },
    XattrPrefix {
        prefix: "system.richacl",
        name_index: EXT4_XATTR_INDEX_RICHACL,
    },
];

/// 从完整属性名中提取命名空间索引和属性名
///
/// 对应 lwext4 的 `ext4_extract_xattr_name()`
///
/// # 参数
///
/// * `full_name` - 完整的属性名（如 "user.comment"）
///
/// # 返回
///
/// 返回 (name_index, name, name_len)：
/// - `name_index` - 命名空间索引
/// - `name` - 实际的属性名（去除前缀后）
/// - `name_len` - 属性名长度
///
/// 如果未找到匹配的前缀，返回 None
///
/// # 示例
///
/// ```ignore
/// let result = extract_xattr_name("user.comment");
/// assert_eq!(result, Some((1, "comment", 7)));
/// ```
pub fn extract_xattr_name(full_name: &str) -> Option<(u8, &str, usize)> {
    if full_name.is_empty() {
        return None;
    }

    // 遍历前缀表查找匹配
    for entry in PREFIX_TABLE {
        let prefix = entry.prefix;
        let prefix_len = prefix.len();

        // 检查前缀是否匹配
        if full_name.len() >= prefix_len && full_name.starts_with(prefix) {
            // 检查前缀是否要求必须有属性名（以 '.' 结尾的前缀）
            let require_name = prefix.ends_with('.');
            let name_start = prefix_len;
            let name_len = full_name.len() - prefix_len;

            // 如果要求名称但长度为0，则无效
            if require_name && name_len == 0 {
                return None;
            }

            let name_index = entry.name_index;

            // 如果需要名称，返回去除前缀后的部分
            if require_name {
                let name = &full_name[name_start..];
                return Some((name_index, name, name_len));
            } else {
                // 不需要额外名称（特殊系统属性）
                return Some((name_index, "", 0));
            }
        }
    }

    // 未找到匹配的前缀
    None
}

/// 根据命名空间索引获取前缀字符串
///
/// 对应 lwext4 的 `ext4_get_xattr_name_prefix()`
///
/// # 参数
///
/// * `name_index` - 命名空间索引
///
/// # 返回
///
/// 返回 (prefix, prefix_len)，如果找不到则返回 None
///
/// # 示例
///
/// ```ignore
/// let result = get_xattr_name_prefix(1);
/// assert_eq!(result, Some(("user.", 5)));
/// ```
pub fn get_xattr_name_prefix(name_index: u8) -> Option<(&'static str, usize)> {
    for entry in PREFIX_TABLE {
        if entry.name_index == name_index {
            let prefix = entry.prefix;
            return Some((prefix, prefix.len()));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_user_xattr() {
        let result = extract_xattr_name("user.comment");
        assert!(result.is_some());
        let (index, name, len) = result.unwrap();
        assert_eq!(index, EXT4_XATTR_INDEX_USER);
        assert_eq!(name, "comment");
        assert_eq!(len, 7);
    }

    #[test]
    fn test_extract_security_xattr() {
        let result = extract_xattr_name("security.selinux");
        assert!(result.is_some());
        let (index, name, len) = result.unwrap();
        assert_eq!(index, EXT4_XATTR_INDEX_SECURITY);
        assert_eq!(name, "selinux");
        assert_eq!(len, 7);
    }

    #[test]
    fn test_extract_empty_name() {
        // "user." 后面没有名称，应该失败
        let result = extract_xattr_name("user.");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_invalid_prefix() {
        let result = extract_xattr_name("invalid.name");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_prefix() {
        let result = get_xattr_name_prefix(EXT4_XATTR_INDEX_USER);
        assert!(result.is_some());
        let (prefix, len) = result.unwrap();
        assert_eq!(prefix, "user.");
        assert_eq!(len, 5);
    }

    #[test]
    fn test_get_prefix_not_found() {
        let result = get_xattr_name_prefix(255);
        assert!(result.is_none());
    }
}
