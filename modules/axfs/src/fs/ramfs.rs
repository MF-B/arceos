use alloc::string::String;
pub use axfs_ramfs::*;
use axfs_vfs::{VfsError, VfsNodeAttr, VfsNodeOps, VfsNodePerm, VfsNodeType, VfsResult};

/// `InterruptFile` is a virtual file node that provides IRQ statistics in RAMFS.
/// path: `/proc/interrupts`
pub struct InterruptFile;

impl VfsNodeOps for InterruptFile {
    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        Ok(VfsNodeAttr::new(
            VfsNodePerm::from_bits_truncate(0o444),
            VfsNodeType::File,
            0,
            0,
        ))
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let mut output = String::new();

        #[cfg(feature = "irq")]
        {
            let counts = axhal::irq::get_all_irq_counts();
            for (irq_num, count) in counts.iter().enumerate() {
                if *count > 0 {
                    output.push_str(&alloc::format!("{irq_num}:\t\t{count}\n"));
                }
            }
        }

        if output.is_empty() {
            output.push_str("No IRQ activity detected\n");
        }

        let bytes = output.as_bytes();
        let available_len = bytes.len().saturating_sub(offset as usize);
        let copy_len = core::cmp::min(buf.len(), available_len);

        if copy_len > 0 && offset < bytes.len() as u64 {
            buf[..copy_len].copy_from_slice(&bytes[offset as usize..offset as usize + copy_len]);
        }

        Ok(copy_len)
    }

    fn write_at(&self, _offset: u64, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::PermissionDenied)
    }

    fn truncate(&self, _size: u64) -> VfsResult {
        Err(VfsError::Unsupported)
    }

    axfs_vfs::impl_vfs_non_dir_default! {}
}
