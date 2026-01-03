#![no_std]
#![allow(clippy::missing_safety_doc)]

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ptr::NonNull;

/// 节点链接接口
/// 用户结构体需要包含特定的 Link 结构（RbLink 或 SplayLink），
/// 并通过实现此 Trait 告诉容器如何访问该 Link。
pub trait TreeAdapter<T> {
    type Link;
    /// 获取节点的 Link 字段的可变指针
    /// # Safety
    /// 指针必须有效
    unsafe fn get_link(node: NonNull<T>) -> NonNull<Self::Link>;
}

// ==========================================
// Splay Tree 实现 (Top-Down Splay)
// ==========================================

pub mod splay {
    use super::*;

    /// Splay Tree 的节点链接字段
    #[derive(Debug)]
    pub struct SplayLink<T> {
        pub left: Option<NonNull<T>>,
        pub right: Option<NonNull<T>>,
    }

    impl<T> Default for SplayLink<T> {
        fn default() -> Self {
            Self { left: None, right: None }
        }
    }

    pub struct SplayTree<T, A>
    where
        A: TreeAdapter<T, Link = SplayLink<T>>,
    {
        root: Option<NonNull<T>>,
        _marker: PhantomData<A>,
    }

    impl<T, A> SplayTree<T, A>
    where
        A: TreeAdapter<T, Link = SplayLink<T>>,
    {
        pub const fn new() -> Self {
            Self {
                root: None,
                _marker: PhantomData,
            }
        }

        pub fn is_empty(&self) -> bool {
            self.root.is_none()
        }

        /// 核心 Splay 操作：将与 key 匹配或最接近的节点旋转到根部
        /// 实现了 BSD tree.h 中的 Top-Down Splay
        unsafe fn splay<F>(&mut self, cmp_fn: F)
        where
            F: Fn(NonNull<T>) -> Ordering,
        {
            let mut root = match self.root {
                Some(r) => r,
                None => return,
            };

            // 临时节点头部，用于连接左右子树
            let mut header = SplayLink::<T>::default();
            // left_tree_max 指向左子树中最大的节点（挂载点）
            // right_tree_min 指向右子树中最小的节点（挂载点）
            // 初始时都指向 header，header.right 代表左子树的根，header.left 代表右子树的根
            // 注意：BSD 实现中采用了比较巧妙的临时变量复用，这里为了清晰，逻辑对应 tree.h
            
            // 为了模拟 C 的取地址操作，我们需要构造指向 header 左右字段的指针
            // 在 Rust 中直接操作栈上变量的指针比较 tricky，我们用变量追踪当前挂载点
            let mut left_tree_max = &mut header.right as *mut Option<NonNull<T>>; 
            let mut right_tree_min = &mut header.left as *mut Option<NonNull<T>>;

            let mut t = root;
            
            // 先清空 header
            header.left = None;
            header.right = None;

            loop {
                let ordering = cmp_fn(t);
                let t_link = A::get_link(t).as_mut();

                match ordering {
                    Ordering::Less => {
                        // 目标在左边
                        if let Some(mut left) = t_link.left {
                            let left_link = A::get_link(left).as_mut();
                            // Zig-Zig (Rotate Right)
                            if cmp_fn(left) == Ordering::Less {
                                t_link.left = left_link.right;
                                left_link.right = Some(t);
                                t = left;
                                if t_link.left.is_none() {
                                    break;
                                }
                            }
                            
                            // Link Right: 当前 t 节点及其右子树挂到右子树集合的最小处
                            // 也就是当前 t 比目标大，放到右边的集合里
                            (*right_tree_min) = Some(t);
                            // 更新 right_tree_min 为 t 的左孩子位置（也就是新的挂载点）
                            right_tree_min = &mut A::get_link(t).as_mut().left as *mut _;
                            t = match A::get_link(t).as_ref().left {
                                Some(n) => n,
                                None => break, // Should actully be handled by zig-zig check
                            };
                        } else {
                            break;
                        }
                    }
                    Ordering::Greater => {
                        // 目标在右边
                        if let Some(mut right) = t_link.right {
                            let right_link = A::get_link(right).as_mut();
                            // Zag-Zag (Rotate Left)
                            if cmp_fn(right) == Ordering::Greater {
                                t_link.right = right_link.left;
                                right_link.left = Some(t);
                                t = right;
                                if t_link.right.is_none() {
                                    break;
                                }
                            }

                            // Link Left
                            (*left_tree_max) = Some(t);
                            left_tree_max = &mut A::get_link(t).as_mut().right as *mut _;
                            t = match A::get_link(t).as_ref().right {
                                Some(n) => n,
                                None => break,
                            };
                        } else {
                            break;
                        }
                    }
                    Ordering::Equal => break,
                }
            }

            // Assemble
            let t_link = A::get_link(t).as_mut();
            (*left_tree_max) = t_link.left;
            (*right_tree_min) = t_link.right;

            t_link.left = header.right;
            t_link.right = header.left;

            self.root = Some(t);
        }

        /// 插入节点
        /// # Safety
        /// `node` 必须是有效指针，且未插入其他树中
        pub unsafe fn insert<F>(&mut self, node: NonNull<T>, cmp_fn: F) -> Option<NonNull<T>>
        where
            F: Fn(NonNull<T>, NonNull<T>) -> Ordering,
        {
            if self.root.is_none() {
                let link = A::get_link(node).as_mut();
                link.left = None;
                link.right = None;
                self.root = Some(node);
                return None;
            }

            // Splay 之后，root 要么是目标值，要么是最接近的值
            self.splay(|n| cmp_fn(node, n));
            
            let root = self.root.unwrap();
            let ordering = cmp_fn(node, root);

            if ordering == Ordering::Equal {
                // 重复元素，返回冲突的 root
                return Some(root);
            }

            let node_link = A::get_link(node).as_mut();
            let root_link = A::get_link(root).as_mut();

            if ordering == Ordering::Less {
                node_link.left = root_link.left;
                node_link.right = Some(root);
                root_link.left = None;
            } else {
                node_link.right = root_link.right;
                node_link.left = Some(root);
                root_link.right = None;
            }

            self.root = Some(node);
            None
        }

        /// 查找节点
        pub unsafe fn find<F>(&mut self, key_cmp: F) -> Option<NonNull<T>>
        where
            F: Fn(NonNull<T>) -> Ordering,
        {
            if self.root.is_none() {
                return None;
            }
            self.splay(&key_cmp);
            let root = self.root.unwrap();
            if key_cmp(root) == Ordering::Equal {
                Some(root)
            } else {
                None
            }
        }
        
        // Remove 逻辑相对复杂，限于篇幅，文件系统中 RB Tree 更常见，重点实现 RB Tree。
    }
}

// ==========================================
// Red-Black Tree 实现
// ==========================================

pub mod rbtree {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RbColor {
        Black,
        Red,
    }

    /// RB Tree 的节点链接字段
    #[derive(Debug)]
    pub struct RbLink<T> {
        pub left: Option<NonNull<T>>,
        pub right: Option<NonNull<T>>,
        pub parent: Option<NonNull<T>>,
        pub color: RbColor,
    }

    impl<T> Default for RbLink<T> {
        fn default() -> Self {
            Self {
                left: None,
                right: None,
                parent: None,
                color: RbColor::Black, // 默认为黑，插入时会被初始化
            }
        }
    }

    pub struct RbTree<T, A>
    where
        A: TreeAdapter<T, Link = RbLink<T>>,
    {
        root: Option<NonNull<T>>,
        _marker: PhantomData<A>,
    }

    impl<T, A> RbTree<T, A>
    where
        A: TreeAdapter<T, Link = RbLink<T>>,
    {
        pub const fn new() -> Self {
            Self {
                root: None,
                _marker: PhantomData,
            }
        }

        // --- 辅助函数 (模拟宏) ---

        #[inline]
        unsafe fn parent(node: NonNull<T>) -> Option<NonNull<T>> {
            A::get_link(node).as_ref().parent
        }

        #[inline]
        unsafe fn set_parent(node: NonNull<T>, parent: Option<NonNull<T>>) {
            A::get_link(node).as_mut().parent = parent;
        }

        #[inline]
        unsafe fn left(node: NonNull<T>) -> Option<NonNull<T>> {
            A::get_link(node).as_ref().left
        }

        #[inline]
        unsafe fn set_left(node: NonNull<T>, child: Option<NonNull<T>>) {
            A::get_link(node).as_mut().left = child;
        }

        #[inline]
        unsafe fn right(node: NonNull<T>) -> Option<NonNull<T>> {
            A::get_link(node).as_ref().right
        }

        #[inline]
        unsafe fn set_right(node: NonNull<T>, child: Option<NonNull<T>>) {
            A::get_link(node).as_mut().right = child;
        }

        #[inline]
        unsafe fn color(node: NonNull<T>) -> RbColor {
            A::get_link(node).as_ref().color
        }

        #[inline]
        unsafe fn set_color(node: NonNull<T>, c: RbColor) {
            A::get_link(node).as_mut().color = c;
        }

        // --- 旋转操作 ---

        unsafe fn rotate_left(&mut self, mut elm: NonNull<T>) {
            let mut tmp = Self::right(elm).expect("Rotate left expects right child");
            
            Self::set_right(elm, Self::left(tmp));
            if let Some(left_of_tmp) = Self::left(tmp) {
                Self::set_parent(left_of_tmp, Some(elm));
            }

            Self::set_parent(tmp, Self::parent(elm));
            
            if let Some(parent) = Self::parent(elm) {
                if Some(elm) == Self::left(parent) {
                    Self::set_left(parent, Some(tmp));
                } else {
                    Self::set_right(parent, Some(tmp));
                }
            } else {
                self.root = Some(tmp);
            }

            Self::set_left(tmp, Some(elm));
            Self::set_parent(elm, Some(tmp));
        }

        unsafe fn rotate_right(&mut self, mut elm: NonNull<T>) {
            let mut tmp = Self::left(elm).expect("Rotate right expects left child");

            Self::set_left(elm, Self::right(tmp));
            if let Some(right_of_tmp) = Self::right(tmp) {
                Self::set_parent(right_of_tmp, Some(elm));
            }

            Self::set_parent(tmp, Self::parent(elm));

            if let Some(parent) = Self::parent(elm) {
                if Some(elm) == Self::left(parent) {
                    Self::set_left(parent, Some(tmp));
                } else {
                    Self::set_right(parent, Some(tmp));
                }
            } else {
                self.root = Some(tmp);
            }

            Self::set_right(tmp, Some(elm));
            Self::set_parent(elm, Some(tmp));
        }

        // --- 插入与修复 ---

        unsafe fn insert_color(&mut self, mut elm: NonNull<T>) {
            let mut parent: NonNull<T>;
            let mut gparent: NonNull<T>;
            let mut tmp: Option<NonNull<T>>;

            while let Some(p) = Self::parent(elm) {
                parent = p;
                if Self::color(parent) != RbColor::Red {
                    break;
                }

                gparent = Self::parent(parent).expect("Red node must have parent (root is black)");

                if Some(parent) == Self::left(gparent) {
                    tmp = Self::right(gparent);
                    if tmp.is_some() && Self::color(tmp.unwrap()) == RbColor::Red {
                        Self::set_color(tmp.unwrap(), RbColor::Black);
                        Self::set_color(parent, RbColor::Black);
                        Self::set_color(gparent, RbColor::Red);
                        elm = gparent;
                        continue;
                    }
                    if Self::right(parent) == Some(elm) {
                        Self::rotate_left(self, parent);
                        let swap = parent;
                        parent = elm;
                        elm = swap;
                    }
                    Self::set_color(parent, RbColor::Black);
                    Self::set_color(gparent, RbColor::Red);
                    Self::rotate_right(self, gparent);
                } else {
                    tmp = Self::left(gparent);
                    if tmp.is_some() && Self::color(tmp.unwrap()) == RbColor::Red {
                        Self::set_color(tmp.unwrap(), RbColor::Black);
                        Self::set_color(parent, RbColor::Black);
                        Self::set_color(gparent, RbColor::Red);
                        elm = gparent;
                        continue;
                    }
                    if Self::left(parent) == Some(elm) {
                        Self::rotate_right(self, parent);
                        let swap = parent;
                        parent = elm;
                        elm = swap;
                    }
                    Self::set_color(parent, RbColor::Black);
                    Self::set_color(gparent, RbColor::Red);
                    Self::rotate_left(self, gparent);
                }
            }
            if let Some(r) = self.root {
                Self::set_color(r, RbColor::Black);
            }
        }

        /// 插入节点
        /// # Safety
        /// 指针有效且无别名冲突
        pub unsafe fn insert<F>(&mut self, elm: NonNull<T>, cmp: F) -> Option<NonNull<T>>
        where
            F: Fn(NonNull<T>, NonNull<T>) -> Ordering,
        {
            let mut tmp = self.root;
            let mut parent = None;
            let mut comp = Ordering::Equal;

            while let Some(node) = tmp {
                parent = Some(node);
                comp = cmp(elm, node);
                match comp {
                    Ordering::Less => tmp = Self::left(node),
                    Ordering::Greater => tmp = Self::right(node),
                    Ordering::Equal => return Some(node), // Key collision
                }
            }

            // 初始化新节点
            Self::set_parent(elm, parent);
            Self::set_left(elm, None);
            Self::set_right(elm, None);
            Self::set_color(elm, RbColor::Red);

            if let Some(p) = parent {
                if comp == Ordering::Less {
                    Self::set_left(p, Some(elm));
                } else {
                    Self::set_right(p, Some(elm));
                }
            } else {
                self.root = Some(elm);
            }

            self.insert_color(elm);
            None
        }

        // --- 查找 ---

        pub unsafe fn find<F>(&self, key_cmp: F) -> Option<NonNull<T>>
        where
            F: Fn(NonNull<T>) -> Ordering,
        {
            let mut tmp = self.root;
            while let Some(node) = tmp {
                match key_cmp(node) {
                    Ordering::Less => tmp = Self::left(node),
                    Ordering::Greater => tmp = Self::right(node),
                    Ordering::Equal => return Some(node),
                }
            }
            None
        }
        
        // --- 移除 ---
        
        unsafe fn remove_color(&mut self, mut parent: Option<NonNull<T>>, mut elm: Option<NonNull<T>>) {
            let mut tmp: NonNull<T>;
            
            while (elm.is_none() || Self::color(elm.unwrap()) == RbColor::Black) && elm != self.root {
                let p_ptr = parent.unwrap(); // Should not be null if elm is not root
                
                if Self::left(p_ptr) == elm {
                    tmp = Self::right(p_ptr).expect("Sibling must exist");
                    if Self::color(tmp) == RbColor::Red {
                        Self::set_color(tmp, RbColor::Black);
                        Self::set_color(p_ptr, RbColor::Red);
                        Self::rotate_left(self, p_ptr);
                        tmp = Self::right(p_ptr).unwrap();
                    }
                    
                    let left_black = Self::left(tmp).map_or(true, |n| Self::color(n) == RbColor::Black);
                    let right_black = Self::right(tmp).map_or(true, |n| Self::color(n) == RbColor::Black);

                    if left_black && right_black {
                        Self::set_color(tmp, RbColor::Red);
                        elm = Some(p_ptr);
                        parent = Self::parent(p_ptr);
                    } else {
                        if right_black {
                            if let Some(l) = Self::left(tmp) {
                                Self::set_color(l, RbColor::Black);
                            }
                            Self::set_color(tmp, RbColor::Red);
                            Self::rotate_right(self, tmp);
                            tmp = Self::right(p_ptr).unwrap();
                        }
                        Self::set_color(tmp, Self::color(p_ptr));
                        Self::set_color(p_ptr, RbColor::Black);
                        if let Some(r) = Self::right(tmp) {
                            Self::set_color(r, RbColor::Black);
                        }
                        Self::rotate_left(self, p_ptr);
                        elm = self.root;
                        break;
                    }
                } else {
                    // Symmetric to above
                    tmp = Self::left(p_ptr).expect("Sibling must exist");
                    if Self::color(tmp) == RbColor::Red {
                        Self::set_color(tmp, RbColor::Black);
                        Self::set_color(p_ptr, RbColor::Red);
                        Self::rotate_right(self, p_ptr);
                        tmp = Self::left(p_ptr).unwrap();
                    }

                    let left_black = Self::left(tmp).map_or(true, |n| Self::color(n) == RbColor::Black);
                    let right_black = Self::right(tmp).map_or(true, |n| Self::color(n) == RbColor::Black);

                    if left_black && right_black {
                        Self::set_color(tmp, RbColor::Red);
                        elm = Some(p_ptr);
                        parent = Self::parent(p_ptr);
                    } else {
                        if left_black {
                            if let Some(r) = Self::right(tmp) {
                                Self::set_color(r, RbColor::Black);
                            }
                            Self::set_color(tmp, RbColor::Red);
                            Self::rotate_left(self, tmp);
                            tmp = Self::left(p_ptr).unwrap();
                        }
                        Self::set_color(tmp, Self::color(p_ptr));
                        Self::set_color(p_ptr, RbColor::Black);
                        if let Some(l) = Self::left(tmp) {
                            Self::set_color(l, RbColor::Black);
                        }
                        Self::rotate_right(self, p_ptr);
                        elm = self.root;
                        break;
                    }
                }
            }
            if let Some(e) = elm {
                Self::set_color(e, RbColor::Black);
            }
        }

        pub unsafe fn remove(&mut self, mut elm: NonNull<T>) -> NonNull<T> {
            let mut child: Option<NonNull<T>>;
            let mut parent: Option<NonNull<T>>;
            let old = elm;
            let color: RbColor;

            if Self::left(elm).is_none() {
                child = Self::right(elm);
            } else if Self::right(elm).is_none() {
                child = Self::left(elm);
            } else {
                // Two children case
                let mut successor = Self::right(elm).unwrap();
                while let Some(left) = Self::left(successor) {
                    successor = left;
                }
                
                // Swap elm with successor (topology only, pointer adjustment hell)
                // tree.h logic: It replaces `elm` with `successor` in the tree structure
                // then handles the cleanup at successor's original position.
                // 这里的逻辑复刻 C 比较繁琐，为了确保正确性，采用标准的逻辑：
                // 用后继节点替代被删除节点的位置和颜色，然后修复后继节点原来的位置。
                
                let succ_old_right = Self::right(successor);
                let succ_parent = Self::parent(successor);
                color = Self::color(successor); // Save successor's color
                
                child = succ_old_right;
                parent = succ_parent;
                
                // Note: if successor is direct child of elm, parent is elm (which is moving)
                // tree.h handles this carefully.
                
                // 简化起见，我们交换 elm 和 successor 的位置不太容易，
                // 我们直接调整指针让 successor 占据 elm 的位置。
                
                // 1. Unlink successor from its old position
                if let Some(c) = child {
                    Self::set_parent(c, parent);
                }
                if let Some(p) = parent {
                    if Some(successor) == Self::left(p) {
                        Self::set_left(p, child);
                    } else {
                        Self::set_right(p, child);
                    }
                } // else: successor was root? No, elm was an ancestor.

                // Special case: if successor was direct child of elm
                if parent == Some(elm) {
                    parent = Some(successor);
                }

                // 2. Put successor in elm's spot
                Self::set_parent(successor, Self::parent(elm));
                Self::set_left(successor, Self::left(elm));
                Self::set_right(successor, Self::right(elm));
                Self::set_color(successor, Self::color(elm)); // Inherit elm's color

                if let Some(p) = Self::parent(elm) {
                    if Some(elm) == Self::left(p) {
                        Self::set_left(p, Some(successor));
                    } else {
                        Self::set_right(p, Some(successor));
                    }
                } else {
                    self.root = Some(successor);
                }

                if let Some(l) = Self::left(elm) {
                    Self::set_parent(l, Some(successor));
                }
                if let Some(r) = Self::right(elm) {
                    Self::set_parent(r, Some(successor));
                }

                if color == RbColor::Black {
                    self.remove_color(parent, child);
                }
                return old;
            }

            // One or zero child case
            parent = Self::parent(elm);
            color = Self::color(elm);

            if let Some(c) = child {
                Self::set_parent(c, parent);
            }
            if let Some(p) = parent {
                if Some(elm) == Self::left(p) {
                    Self::set_left(p, child);
                } else {
                    Self::set_right(p, child);
                }
            } else {
                self.root = child;
            }

            if color == RbColor::Black {
                self.remove_color(parent, child);
            }
            
            old
        }
    }
}

// ==========================================
// 示例用法
// ==========================================

#[cfg(not(feature = "std"))] // Just for demonstration
pub mod example {
    use super::*;
    use super::rbtree::*;

    // 假设这是文件系统中的 Inode 结构
    pub struct Inode {
        pub id: u64,
        pub size: u64,
        // 侵入式链接字段
        rb_link: RbLink<Inode>,
    }

    impl Inode {
        pub fn new(id: u64) -> Self {
            Self {
                id,
                size: 0,
                rb_link: RbLink::default(),
            }
        }
    }

    // 定义 Adapter，告诉 Tree 如何找到 rb_link
    pub struct InodeTreeAdapter;

    impl TreeAdapter<Inode> for InodeTreeAdapter {
        type Link = RbLink<Inode>;

        unsafe fn get_link(node: NonNull<Inode>) -> NonNull<Self::Link> {
            // 计算 rb_link 字段的偏移并返回指针
            // 在实际工程中，建议使用 `memoffset` crate 的 `offset_of!` 宏
            // 这里为了无依赖演示，直接指针转换 (假设 rb_link 是最后一个字段或者通过转换获得)
            // 严谨写法：
            let ptr = node.as_ptr();
            let link_ptr = &mut (*ptr).rb_link as *mut RbLink<Inode>;
            NonNull::new_unchecked(link_ptr)
        }
    }

    // 定义文件系统使用的树类型
    pub type InodeTree = RbTree<Inode, InodeTreeAdapter>;

    pub fn test_fs_tree() {
        unsafe {
            let mut tree = InodeTree::new();

            // 模拟内核分配 (通常来自 Slab)
            let mut node1 = Inode::new(100);
            let mut node2 = Inode::new(50);
            let mut node3 = Inode::new(150);

            let p1 = NonNull::new_unchecked(&mut node1 as *mut _);
            let p2 = NonNull::new_unchecked(&mut node2 as *mut _);
            let p3 = NonNull::new_unchecked(&mut node3 as *mut _);

            // 插入
            tree.insert(p1, |a, b| a.as_ref().id.cmp(&b.as_ref().id));
            tree.insert(p2, |a, b| a.as_ref().id.cmp(&b.as_ref().id));
            tree.insert(p3, |a, b| a.as_ref().id.cmp(&b.as_ref().id));

            // 查找
            let target = tree.find(|n| 50.cmp(&n.as_ref().id));
            assert!(target.is_some());
            assert_eq!(target.unwrap().as_ref().id, 50);
        }
    }
}