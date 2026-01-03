//! 块设备抽象
//!
//! 提供块设备接口和块级 I/O 操作。
//! block/device.rs 提供函数直接调用device接口对设备进行读写, 也提供了一些cache管理接口

//! block/io.rs 提供函数交互cache的读写，读写都先操作cache,若没有对应块的cache则调用设备接口载入cache， 如果没有启用cache, 则直接磁盘和vec buffer
//! 但io.rs所有函数都需要提供vec buffer, 从vec buffer读入数据写到cache 或者从cache读出数据写到vec buffer

//! block/handle 可以提供对某块cache的引用， 保证一致性 

mod device;
mod io;
mod handle;
mod lock;

pub use device::{BlockDevice, BlockDev};
pub use handle::Block;
pub use lock::{DeviceLock, NoLock};
