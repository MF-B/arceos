//! Root directory of the filesystem
//!
//! TODO: it doesn't work very well if the mount points have containment relationships.

use alloc::{format, string::String, string::ToString, sync::Arc, vec::Vec};
use axerrno::{AxError, AxResult, ax_err};
use axfs_vfs::{VfsNodeAttr, VfsNodeOps, VfsNodePerm, VfsNodeRef, VfsNodeType, VfsOps, VfsResult};
use axns::{ResArc, def_resource};
use axsync::Mutex;
use lazyinit::LazyInit;
use spin::RwLock;

use crate::{
    api::FileType,
    fs::{self},
    mounts,
};

def_resource! {
    pub static CURRENT_DIR_PATH: ResArc<Mutex<String>> = ResArc::new();
    pub static CURRENT_DIR: ResArc<Mutex<VfsNodeRef>> = ResArc::new();
}

impl CURRENT_DIR_PATH {
    /// Return a copy of the inner path.
    pub fn copy_inner(&self) -> Mutex<String> {
        Mutex::new(self.lock().clone())
    }
}

impl CURRENT_DIR {
    /// Return a copy of the CURRENT_DIR_NODE.
    pub fn copy_inner(&self) -> Mutex<VfsNodeRef> {
        Mutex::new(self.lock().clone())
    }
}

struct MountPoint {
    path: &'static str,
    fs: Arc<dyn VfsOps>,
}

struct RootDirectory {
    main_fs: Arc<dyn VfsOps>,
    mounts: RwLock<Vec<MountPoint>>,
}

static ROOT_DIR: LazyInit<Arc<RootDirectory>> = LazyInit::new();

impl MountPoint {
    pub fn new(path: &'static str, fs: Arc<dyn VfsOps>) -> Self {
        Self { path, fs }
    }
}

impl Drop for MountPoint {
    fn drop(&mut self) {
        self.fs.umount().ok();
    }
}

impl RootDirectory {
    pub const fn new(main_fs: Arc<dyn VfsOps>) -> Self {
        Self {
            main_fs,
            mounts: RwLock::new(Vec::new()),
        }
    }

    pub fn mount(&self, path: &'static str, fs: Arc<dyn VfsOps>) -> AxResult {
        if path == "/" {
            return ax_err!(InvalidInput, "cannot mount root filesystem");
        }
        if !path.starts_with('/') {
            return ax_err!(InvalidInput, "mount path must start with '/'");
        }
        if self.mounts.read().iter().any(|mp| mp.path == path) {
            return ax_err!(InvalidInput, "mount point already exists");
        }
        // create the mount point in the main filesystem if it does not exist
        self.main_fs.root_dir().create(path, FileType::Dir)?;
        fs.mount(path, self.main_fs.root_dir().lookup(path)?)?;
        self.mounts.write().push(MountPoint::new(path, fs));
        Ok(())
    }

    pub fn _umount(&self, path: &str) {
        self.mounts.write().retain(|mp| mp.path != path);
    }

    pub fn contains(&self, path: &str) -> bool {
        self.mounts.read().iter().any(|mp| mp.path == path)
    }

    fn lookup_mounted_fs<F, T>(&self, path: &str, f: F) -> AxResult<T>
    where
        F: FnOnce(Arc<dyn VfsOps>, &str) -> AxResult<T>,
    {
        debug!("lookup at root: {}", path);
        let path = path.trim_matches('/');
        if let Some(rest) = path.strip_prefix("./") {
            return self.lookup_mounted_fs(rest, f);
        }

        let mut idx = 0;
        let mut max_len = 0;

        // Find the filesystem that has the longest mounted path match
        // TODO: more efficient, e.g. trie
        for (i, mp) in self.mounts.read().iter().enumerate() {
            // skip the first '/'
            if path.starts_with(&mp.path[1..]) && mp.path.len() - 1 > max_len {
                max_len = mp.path.len() - 1;
                idx = i;
            }
        }

        if max_len == 0 {
            f(self.main_fs.clone(), path) // not matched any mount point
        } else {
            let rest_path = if path.len() > max_len && path.as_bytes()[max_len] == b'/' {
                &path[max_len + 1..] // skip mount point and the '/'
            } else if path.len() == max_len {
                "" // exact match, empty rest
            } else {
                &path[max_len..] // fallback
            };
            f(self.mounts.read()[idx].fs.clone(), rest_path) // matched at `idx`
        }
    }
}

impl VfsNodeOps for RootDirectory {
    axfs_vfs::impl_vfs_dir_default! {}

    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        self.main_fs.root_dir().get_attr()
    }

    fn lookup(self: Arc<Self>, path: &str) -> VfsResult<VfsNodeRef> {
        self.lookup_mounted_fs(path, |fs, rest_path| fs.root_dir().lookup(rest_path))
    }

    fn create(&self, path: &str, ty: VfsNodeType) -> VfsResult {
        self.lookup_mounted_fs(path, |fs, rest_path| {
            if rest_path.is_empty() {
                Ok(()) // already exists
            } else {
                fs.root_dir().create(rest_path, ty)
            }
        })
    }

    fn remove(&self, path: &str) -> VfsResult {
        self.lookup_mounted_fs(path, |fs, rest_path| {
            if rest_path.is_empty() {
                ax_err!(PermissionDenied) // cannot remove mount points
            } else {
                fs.root_dir().remove(rest_path)
            }
        })
    }

    fn rename(&self, src_path: &str, dst_path: &str) -> VfsResult {
        self.lookup_mounted_fs(src_path, |fs, rest_path| {
            if rest_path.is_empty() {
                ax_err!(PermissionDenied) // cannot rename mount points
            } else {
                fs.root_dir().rename(src_path, dst_path)
            }
        })
    }

    fn symlink(&self, target: &str, path: &str) -> VfsResult {
        self.lookup_mounted_fs(path, |fs, rest_path| {
            if rest_path.is_empty() {
                ax_err!(InvalidInput)
            } else {
                fs.root_dir().symlink(target, rest_path)
            }
        })
    }

    fn readlink(&self, path: &str, buf: &mut [u8]) -> VfsResult<usize> {
        self.lookup_mounted_fs(path, |fs, rest_path| {
            if rest_path.is_empty() {
                ax_err!(NotFound) // cannot read link of mount points
            } else {
                fs.root_dir().readlink(path, buf)
            }
        })
    }
}

pub(crate) fn init_rootfs(disk: crate::dev::Disk) {
    cfg_if::cfg_if! {
        if #[cfg(feature = "myfs")] { // override the default filesystem
            let main_fs = fs::myfs::new_myfs(disk);
        } else if #[cfg(feature = "lwext4_rs")] {
            static EXT4_FS: LazyInit<Arc<fs::lwext4_rust::Ext4FileSystem>> = LazyInit::new();
            EXT4_FS.init_once(Arc::new(fs::lwext4_rust::Ext4FileSystem::new(disk)));
            let main_fs = EXT4_FS.clone();
        } else if #[cfg(feature = "fatfs")] {
            static FAT_FS: LazyInit<Arc<fs::fatfs::FatFileSystem>> = LazyInit::new();
            FAT_FS.init_once(Arc::new(fs::fatfs::FatFileSystem::new(disk)));
            FAT_FS.init();
            let main_fs = FAT_FS.clone();
        }
    }

    let root_dir = RootDirectory::new(main_fs);

    #[cfg(feature = "devfs")]
    root_dir
        .mount("/dev", mounts::devfs())
        .expect("failed to mount devfs at /dev");

    #[cfg(feature = "ramfs")]
    root_dir
        .mount("/tmp", mounts::ramfs())
        .expect("failed to mount ramfs at /tmp");

    // Mount another ramfs as procfs
    #[cfg(feature = "procfs")]
    root_dir // should not fail
        .mount("/proc", mounts::procfs().unwrap())
        .expect("fail to mount procfs at /proc");

    // Mount another ramfs as sysfs
    #[cfg(feature = "sysfs")]
    root_dir // should not fail
        .mount("/sys", mounts::sysfs().unwrap())
        .expect("fail to mount sysfs at /sys");

    ROOT_DIR.init_once(Arc::new(root_dir));
    CURRENT_DIR.init_new(Mutex::new(ROOT_DIR.clone()));
    CURRENT_DIR_PATH.init_new(Mutex::new("/".into()));
}

fn parent_node_of(dir: Option<&VfsNodeRef>, path: &str) -> VfsNodeRef {
    if path.starts_with('/') {
        ROOT_DIR.clone()
    } else {
        dir.cloned().unwrap_or_else(|| CURRENT_DIR.lock().clone())
    }
}

pub(crate) fn absolute_path(path: &str) -> AxResult<String> {
    if path.starts_with('/') {
        Ok(axfs_vfs::path::canonicalize(path))
    } else {
        let path = CURRENT_DIR_PATH.lock().clone() + path;
        Ok(axfs_vfs::path::canonicalize(&path))
    }
}

pub(crate) fn lookup(dir: Option<&VfsNodeRef>, path: &str) -> AxResult<VfsNodeRef> {
    if path.is_empty() {
        return ax_err!(NotFound);
    }
    let node = parent_node_of(dir, path).lookup(path)?;
    if path.ends_with('/') && !node.get_attr()?.is_dir() {
        ax_err!(NotADirectory)
    } else {
        Ok(node)
    }
}

/// Lookup a path and follow symbolic links to get the final target
pub(crate) fn lookup_follow_symlinks_public(
    dir: Option<&VfsNodeRef>,
    path: &str,
) -> AxResult<VfsNodeRef> {
    const MAX_SYMLINK_DEPTH: u32 = 8;

    let mut current_path = String::from(path);
    let mut depth = 0;

    loop {
        if depth > MAX_SYMLINK_DEPTH {
            return ax_err!(InvalidInput, "Too many levels of symbolic links");
        }

        match lookup(dir, &current_path) {
            Ok(node) => {
                if !node.is_symlink() {
                    return Ok(node);
                }

                // Read symlink target
                let mut buf = [0u8; 4096];
                let target_len = node.readlink(&current_path, &mut buf)?;
                let target =
                    core::str::from_utf8(&buf[..target_len]).map_err(|_| AxError::InvalidData)?;

                current_path = if target.starts_with('/') {
                    target.to_string()
                } else {
                    // Handle relative symlinks
                    if let Some(parent_pos) = current_path.rfind('/') {
                        format!("{}/{}", &current_path[..parent_pos], target)
                    } else {
                        target.to_string()
                    }
                };
                depth += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

pub(crate) fn create_file(dir: Option<&VfsNodeRef>, path: &str) -> AxResult<VfsNodeRef> {
    if path.is_empty() {
        return ax_err!(NotFound);
    } else if path.ends_with('/') {
        return ax_err!(NotADirectory);
    }
    let parent = parent_node_of(dir, path);
    parent.create(path, VfsNodeType::File)?;
    parent.lookup(path)
}

pub(crate) fn create_dir(dir: Option<&VfsNodeRef>, path: &str) -> AxResult {
    match lookup(dir, path) {
        Ok(_) => ax_err!(AlreadyExists),
        Err(AxError::NotFound) => parent_node_of(dir, path).create(path, VfsNodeType::Dir),
        Err(e) => Err(e),
    }
}

pub(crate) fn remove_file(dir: Option<&VfsNodeRef>, path: &str) -> AxResult {
    let node = lookup(dir, path)?;
    let attr = node.get_attr()?;
    if attr.is_dir() {
        ax_err!(IsADirectory)
    } else if !attr.perm().owner_writable() {
        ax_err!(PermissionDenied)
    } else {
        parent_node_of(dir, path).remove(path)
    }
}

pub(crate) fn remove_dir(dir: Option<&VfsNodeRef>, path: &str) -> AxResult {
    if path.is_empty() {
        return ax_err!(NotFound);
    }
    let path_check = path.trim_matches('/');
    if path_check.is_empty() {
        return ax_err!(DirectoryNotEmpty); // rm -d '/'
    } else if path_check == "."
        || path_check == ".."
        || path_check.ends_with("/.")
        || path_check.ends_with("/..")
    {
        return ax_err!(InvalidInput);
    }
    if ROOT_DIR.contains(&absolute_path(path)?) {
        return ax_err!(PermissionDenied);
    }

    let node = lookup(dir, path)?;
    let attr = node.get_attr()?;
    if !attr.is_dir() {
        ax_err!(NotADirectory)
    } else if !attr.perm().owner_writable() {
        ax_err!(PermissionDenied)
    } else {
        parent_node_of(dir, path).remove(path)
    }
}

pub(crate) fn current_dir() -> AxResult<String> {
    Ok(CURRENT_DIR_PATH.lock().clone())
}

pub(crate) fn set_current_dir(path: &str) -> AxResult {
    let mut abs_path = absolute_path(path)?;
    if !abs_path.ends_with('/') {
        abs_path += "/";
    }
    if abs_path == "/" {
        *CURRENT_DIR.lock() = ROOT_DIR.clone();
        *CURRENT_DIR_PATH.lock() = "/".into();
        return Ok(());
    }

    let node = lookup(None, &abs_path)?;
    let attr = node.get_attr()?;
    if !attr.is_dir() {
        ax_err!(NotADirectory)
    } else if !attr.perm().owner_executable() {
        ax_err!(PermissionDenied)
    } else {
        *CURRENT_DIR.lock() = node;
        *CURRENT_DIR_PATH.lock() = abs_path;
        Ok(())
    }
}

pub(crate) fn rename(old: &str, new: &str) -> AxResult {
    if parent_node_of(None, new).lookup(new).is_ok() {
        warn!("dst file already exist, now remove it");
        remove_file(None, new)?;
    }
    parent_node_of(None, old).rename(old, new)
}

pub(crate) fn create_symlink(target: &str, path: &str) -> AxResult {
    if target.is_empty() || path.is_empty() {
        return ax_err!(InvalidInput);
    }

    // For EXT4, try using the root directory directly with full path
    ROOT_DIR.main_fs.root_dir().symlink(target, path)
}

pub(crate) fn read_link(path: &str, buf: &mut [u8]) -> AxResult<usize> {
    if path.is_empty() {
        return ax_err!(NotFound);
    }

    // Try EXT4 filesystem first for regular symlinks
    ROOT_DIR
        .main_fs
        .root_dir()
        .readlink(path, buf)
        .or_else(|_| {
            // Fallback to VFS layer for mount points (procfs, ramfs, etc.)
            lookup(None, path)?.readlink("", buf)
        })
}

pub(crate) fn set_perm(path: &str, mode: u16) -> AxResult {
    let abs_path = absolute_path(path)?;
    let node = lookup(None, &abs_path)?;
    let mut attr = node.get_attr()?;
    attr.set_perm(VfsNodePerm::from_bits(mode).ok_or(AxError::InvalidInput)?);
    Ok(())
}

pub(crate) fn is_symlink(path: &str) -> AxResult<bool> {
    if path.is_empty() {
        return ax_err!(NotFound);
    }
    let node = lookup(None, path)?;
    Ok(node.is_symlink())
}

pub(crate) fn add_node(dir: Option<&VfsNodeRef>, path: &'static str, ty: VfsNodeRef) -> AxResult {
    if path.is_empty() {
        return ax_err!(NotFound);
    }
    let parent = parent_node_of(dir, path);
    parent.add_node(path, ty)
}
