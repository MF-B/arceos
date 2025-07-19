pub use axfs_devfs::*;
use axhal::console::{read_bytes, write_bytes};

use axfs_vfs::{VfsNodeAttr, VfsNodeOps, VfsNodePerm, VfsNodeType, VfsResult};

/// `TtyDev` is a virtual character device node for console I/O.
/// path: `/dev/tty`
pub struct TtyDev;

impl VfsNodeOps for TtyDev {
    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        Ok(VfsNodeAttr::new(
            VfsNodePerm::default_file(),
            VfsNodeType::CharDevice,
            0,
            0,
        ))
    }

    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let n = read_bytes(buf);
        Ok(n)
    }

    fn write_at(&self, _offset: u64, buf: &[u8]) -> VfsResult<usize> {
        write_bytes(buf);
        Ok(buf.len())
    }

    fn truncate(&self, _size: u64) -> VfsResult {
        Ok(())
    }

    axfs_vfs::impl_vfs_non_dir_default! {}
}
