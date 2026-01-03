//! Transaction-aware block handle
//!
//! Provides automatic dirty block tracking for transactions.
//!
//! This module implements the RAII pattern for transaction blocks:
//! when you modify a block through `with_data_mut()`, it automatically
//! tracks the modification. When the block handle is dropped, it
//! automatically adds the block to the transaction's dirty list if needed.
//!
//! This eliminates the error-prone manual `mark_dirty()` pattern.

use crate::{
    block::{Block, BlockDevice, BlockDev},
    error::Result,
};
use alloc::rc::Rc;
use core::cell::RefCell;

/// Transaction-aware block handle
///
/// Automatically tracks block modifications and adds blocks to the
/// transaction's dirty list on drop.
///
/// # Design
///
/// Uses `Rc<RefCell<Vec<u64>>>` to share the dirty blocks list between
/// the transaction and block handles. This allows:
/// - Block handle to mark dirty without holding transaction reference
/// - Multiple block handles (though Rust prevents simultaneous mutable access)
/// - Clean RAII semantics with automatic dirty tracking
///
/// # Example
///
/// ```rust,ignore
/// let mut trans = SimpleTransaction::begin(&mut bdev)?;
///
/// // Old error-prone pattern (still supported):
/// let mut block = trans.get_block(100)?;
/// block.with_data_mut(|data| data[0] = 0x42)?;
/// trans.mark_dirty(100)?;  // Easy to forget!
///
/// // New automatic pattern:
/// let mut block = trans.get_block_tracked(100)?;
/// block.with_data_mut(|data| data[0] = 0x42)?;
/// // Block automatically marked dirty on drop!
/// ```
pub struct TransactionBlock<'a, D: BlockDevice> {
    /// Underlying block handle
    inner: Block<'a, D>,

    /// Logical block address
    lba: u64,

    /// Whether this block was modified
    modified: bool,

    /// Shared reference to transaction's dirty blocks list
    dirty_blocks: Rc<RefCell<alloc::vec::Vec<u64>>>,
}

impl<'a, D: BlockDevice> TransactionBlock<'a, D> {
    /// Create a new transaction block handle
    ///
    /// # Parameters
    ///
    /// * `inner` - The underlying block handle
    /// * `lba` - Logical block address
    /// * `dirty_blocks` - Shared reference to transaction's dirty list
    ///
    /// # Returns
    ///
    /// New transaction block handle
    pub(super) fn new(
        inner: Block<'a, D>,
        lba: u64,
        dirty_blocks: Rc<RefCell<alloc::vec::Vec<u64>>>,
    ) -> Self {
        Self {
            inner,
            lba,
            modified: false,
            dirty_blocks,
        }
    }

    /// Get the logical block address
    pub fn lba(&self) -> u64 {
        self.lba
    }

    /// Access block data (read-only)
    ///
    /// Does not mark block as modified.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// block.with_data(|data| {
    ///     println!("First byte: {:02x}", data[0]);
    /// })?;
    /// ```
    pub fn with_data<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> R,
    {
        self.inner.with_data(f)
    }

    /// Access block data (mutable)
    ///
    /// Automatically marks block as modified. When this handle is dropped,
    /// the block will be added to the transaction's dirty list.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// block.with_data_mut(|data| {
    ///     data[0] = 0x42;
    ///     data[1] = 0x43;
    /// })?;
    /// // Block automatically marked dirty when handle drops
    /// ```
    pub fn with_data_mut<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let result = self.inner.with_data_mut(f)?;
        self.modified = true;
        Ok(result)
    }

    /// Manually release the block
    ///
    /// Usually not needed - Drop trait handles this automatically.
    pub fn release(self) -> Result<()> {
        // Drop will handle the dirty tracking
        Ok(())
    }
}

impl<'a, D: BlockDevice> Drop for TransactionBlock<'a, D> {
    /// Automatically mark block dirty in transaction if modified
    fn drop(&mut self) {
        if self.modified {
            let mut dirty_blocks = self.dirty_blocks.borrow_mut();

            // Only add if not already in the list
            if !dirty_blocks.contains(&self.lba) {
                dirty_blocks.push(self.lba);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockDevice;
    use crate::error::Result;
    use alloc::vec::Vec;

    struct MockDevice {
        block_size: u32,
        sector_size: u32,
        total_blocks: u64,
        storage: Vec<u8>,
    }

    impl MockDevice {
        fn new(total_blocks: u64) -> Self {
            let block_size = 4096;
            let sector_size = 512;
            let storage = alloc::vec![0u8; (total_blocks * block_size as u64) as usize];
            Self {
                block_size,
                sector_size,
                total_blocks,
                storage,
            }
        }
    }

    impl BlockDevice for MockDevice {
        fn block_size(&self) -> u32 {
            self.block_size
        }

        fn sector_size(&self) -> u32 {
            self.sector_size
        }

        fn total_blocks(&self) -> u64 {
            self.total_blocks
        }

        fn read_blocks(&mut self, lba: u64, count: u32, buf: &mut [u8]) -> Result<usize> {
            let start = (lba * self.sector_size as u64) as usize;
            let len = (count * self.sector_size) as usize;
            if start + len <= self.storage.len() {
                buf[..len].copy_from_slice(&self.storage[start..start + len]);
                Ok(len)
            } else {
                Ok(0)
            }
        }

        fn write_blocks(&mut self, lba: u64, count: u32, buf: &[u8]) -> Result<usize> {
            let start = (lba * self.sector_size as u64) as usize;
            let len = (count * self.sector_size) as usize;
            if start + len <= self.storage.len() {
                self.storage[start..start + len].copy_from_slice(&buf[..len]);
                Ok(len)
            } else {
                Ok(0)
            }
        }
    }

    #[test]
    fn test_transaction_block_automatic_dirty_tracking() {
        let device = MockDevice::new(100);
        let mut bdev = BlockDev::new_with_cache(device, 8).unwrap();
        let dirty_list = Rc::new(RefCell::new(Vec::new()));

        {
            let block = Block::get(&mut bdev, 0).unwrap();
            let mut trans_block = TransactionBlock::new(block, 0, dirty_list.clone());

            // Modify the block
            trans_block
                .with_data_mut(|data| {
                    data[0] = 0x42;
                })
                .unwrap();

            // Dirty list should be empty while block is still held
            assert_eq!(dirty_list.borrow().len(), 0);
        } // Drop happens here

        // After drop, block should be in dirty list
        assert_eq!(dirty_list.borrow().len(), 1);
        assert_eq!(dirty_list.borrow()[0], 0);
    }

    #[test]
    fn test_transaction_block_no_tracking_without_modification() {
        let device = MockDevice::new(100);
        let mut bdev = BlockDev::new_with_cache(device, 8).unwrap();
        let dirty_list = Rc::new(RefCell::new(Vec::new()));

        {
            let block = Block::get(&mut bdev, 0).unwrap();
            let mut trans_block = TransactionBlock::new(block, 0, dirty_list.clone());

            // Only read, don't modify
            trans_block.with_data(|data| {
                let _ = data[0];
            }).unwrap();
        } // Drop happens here

        // Block should NOT be in dirty list (not modified)
        assert_eq!(dirty_list.borrow().len(), 0);
    }

    #[test]
    fn test_transaction_block_no_duplicate_dirty_entries() {
        let device = MockDevice::new(100);
        let mut bdev = BlockDev::new_with_cache(device, 8).unwrap();
        let dirty_list = Rc::new(RefCell::new(Vec::new()));

        {
            let block = Block::get(&mut bdev, 0).unwrap();
            let mut trans_block = TransactionBlock::new(block, 0, dirty_list.clone());
            trans_block.with_data_mut(|data| data[0] = 0x42).unwrap();
        }

        assert_eq!(dirty_list.borrow().len(), 1);

        {
            let block = Block::get(&mut bdev, 0).unwrap();
            let mut trans_block = TransactionBlock::new(block, 0, dirty_list.clone());
            trans_block.with_data_mut(|data| data[0] = 0x43).unwrap();
        }

        // Should still be only one entry (no duplicates)
        assert_eq!(dirty_list.borrow().len(), 1);
        assert_eq!(dirty_list.borrow()[0], 0);
    }
}
