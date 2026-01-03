//! 块 I/O 操作实现

use super::{BlockDev, BlockDevice};
use crate::error::{Error, ErrorKind, Result};
use alloc::vec;

impl<D: BlockDevice> BlockDev<D> {
    /// 读取单个逻辑块
    ///
    /// 从指定逻辑块地址读取一个完整的块到缓冲区。
    /// 如果启用了缓存，优先从缓存读取；缓存未命中则从设备读取并填充缓存。
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    /// * `buf` - 目标缓冲区（大小至少为 block_size）
    ///
    /// # 返回
    ///
    /// 成功返回读取的字节数
    pub fn read_block(&mut self, lba: u64, buf: &mut [u8]) -> Result<usize> {
        let block_size = self.device().block_size();

        if buf.len() < block_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "buffer too small for block",
            ));
        }

        self.inc_read_count();

        // 如果启用了缓存，尝试从缓存读取
        let cache_miss = if let Some(cache) = &self.bcache {
            // 尝试从缓存读取（只读检查）
            match cache.read_block(lba) {
                Ok(data) => {
                    // 缓存命中
                    buf[..data.len()].copy_from_slice(data);
                    return Ok(data.len());
                }
                Err(_) => true, // 缓存未命中
            }
        } else {
            false // 无缓存
        };

        if cache_miss {
            // 缓存未命中 - 从设备读取到用户缓冲区
            let pba = self.logical_to_physical(lba);
            let count = self.sectors_per_block();
            self.device_mut().read_blocks(pba, count, buf)?;

            // 将数据填充到缓存
            if let Some(cache) = &mut self.bcache {
                let (cache_buf, _is_new) = cache.alloc(lba)?;
                cache_buf.data.copy_from_slice(&buf[..block_size as usize]);
                cache_buf.mark_uptodate();
                cache.free(lba)?;
            }

            return Ok(block_size as usize);
        }

        // 无缓存 - 直接从设备读取
        let pba = self.logical_to_physical(lba);
        let count = self.sectors_per_block();
        self.device_mut().read_blocks(pba, count, buf)
    }

    /// 写入单个逻辑块
    ///
    /// 将缓冲区数据写入指定逻辑块地址。
    /// 如果启用了缓存，写入缓存并标记为脏；否则直接写入设备。
    ///
    /// # 参数
    ///
    /// * `lba` - 逻辑块地址
    /// * `buf` - 源数据缓冲区（大小至少为 block_size）
    ///
    /// # 返回
    ///
    /// 成功返回写入的字节数
    pub fn write_block(&mut self, lba: u64, buf: &[u8]) -> Result<usize> {
        let block_size = self.device().block_size();

        if buf.len() < block_size as usize {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "buffer too small for block",
            ));
        }

        self.inc_write_count();

        // 如果启用了缓存，写入缓存
        if let Some(cache) = &mut self.bcache {
            // 先尝试从缓存获取块（可能已存在）
            match cache.write_block(lba, buf) {
                Ok(n) => {
                    // 块已在缓存中，写入成功
                    return Ok(n);
                }
                Err(_) => {
                    // 块不在缓存中 - 分配新块并写入
                    let (cache_buf, _is_new) = cache.alloc(lba)?;
                    cache_buf.data[..buf.len()].copy_from_slice(buf);
                    cache_buf.mark_uptodate();
                    cache_buf.mark_dirty();

                    // 将块加入脏列表
                    cache.mark_dirty(lba)?;

                    // 减少引用计数
                    cache.free(lba)?;

                    return Ok(buf.len());
                }
            }
        }

        // 无缓存 - 直接写入设备
        let pba = self.logical_to_physical(lba);
        let count = self.sectors_per_block();
        self.device_mut().write_blocks(pba, count, buf)
    }

    /// 读取字节
    ///
    /// 从任意字节偏移读取，自动处理跨块情况。
    ///
    /// # 参数
    ///
    /// * `offset` - 字节偏移量
    /// * `buf` - 目标缓冲区
    ///
    /// # 返回
    ///
    /// 成功返回读取的字节数
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut buf = vec![0u8; 100];
    /// block_dev.read_bytes(1024, &mut buf)?;
    /// ```
    pub fn read_bytes(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let len = buf.len();
        let block_size = self.device().block_size() as u64;

        // 计算起始块和块内偏移
        let start_block = offset / block_size;
        let block_offset = (offset % block_size) as usize;

        // 计算需要读取的块数
        let total_size = block_offset + len;
        let block_count = ((total_size as u64 + block_size - 1) / block_size) as usize;

        // 分配临时缓冲区
        let mut temp = vec![0u8; block_count * block_size as usize];

        // 读取所有相关块
        for i in 0..block_count {
            let lba = start_block + i as u64;
            let block_buf = &mut temp[i * block_size as usize..(i + 1) * block_size as usize];
            self.read_block(lba, block_buf)?;
        }

        // 复制所需字节
        buf.copy_from_slice(&temp[block_offset..block_offset + len]);

        Ok(len)
    }

    /// 写入字节
    ///
    /// 向任意字节偏移写入，自动处理跨块情况。
    ///
    /// # 参数
    ///
    /// * `offset` - 字节偏移量
    /// * `buf` - 源数据缓冲区
    ///
    /// # 返回
    ///
    /// 成功返回写入的字节数
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let data = b"Hello, ext4!";
    /// block_dev.write_bytes(1024, data)?;
    /// ```
    pub fn write_bytes(&mut self, offset: u64, buf: &[u8]) -> Result<usize> {
        let len = buf.len();
        let block_size = self.device().block_size() as u64;

        let start_block = offset / block_size;
        let block_offset = (offset % block_size) as usize;

        let total_size = block_offset + len;
        let block_count = ((total_size as u64 + block_size - 1) / block_size) as usize;

        let mut temp = vec![0u8; block_count * block_size as usize];

        // 如果不是块对齐，需要先读取现有数据
        if block_offset != 0 || len % block_size as usize != 0 {
            for i in 0..block_count {
                let lba = start_block + i as u64;
                let block_buf =
                    &mut temp[i * block_size as usize..(i + 1) * block_size as usize];
                // 忽略读取错误（可能是新块）
                let _ = self.read_block(lba, block_buf);
            }
        }

        // 写入数据到临时缓冲区
        temp[block_offset..block_offset + len].copy_from_slice(buf);

        // 写回所有块
        for i in 0..block_count {
            let lba = start_block + i as u64;
            let block_buf = &temp[i * block_size as usize..(i + 1) * block_size as usize];
            self.write_block(lba, block_buf)?;
        }

        Ok(len)
    }

    /// 刷新所有缓存
    ///
    /// 如果启用了缓存，先刷新所有脏块到设备，然后调用设备的 flush。
    /// 这是两层刷新：缓存层和硬件层。
    pub fn flush(&mut self) -> Result<()> {
        // 第一层：刷新缓存中的脏块
        let sector_size = self.device().sector_size();
        let partition_offset = self.partition_offset();

        // 临时取出缓存以避免借用冲突
        if let Some(mut cache) = self.bcache.take() {
            let result = cache.flush_all(self.device_mut(), sector_size, partition_offset);
            // 恢复缓存
            self.bcache = Some(cache);
            result?;
        }

        // 第二层：调用设备的硬件刷新（如 fsync）
        self.device_mut().flush()
    }
}
