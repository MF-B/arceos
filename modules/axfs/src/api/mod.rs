//! [`std::fs`]-like high-level filesystem manipulation operations.

mod dir;
mod file;

pub use self::dir::{DirBuilder, DirEntry, ReadDir};
pub use self::file::{File, FileType, Metadata, OpenOptions, Permissions};

use alloc::{string::String, vec::Vec};
use axio::{self as io, prelude::*};

/// Returns an iterator over the entries within a directory.
pub fn read_dir(path: &str) -> io::Result<ReadDir> {
    ReadDir::new(path)
}

/// Returns the canonical, absolute form of a path with all intermediate
/// components normalized.
pub fn canonicalize(path: &str) -> io::Result<String> {
    crate::root::absolute_path(path)
}

/// Returns the current working directory as a [`String`].
pub fn current_dir() -> io::Result<String> {
    crate::root::current_dir()
}

/// Changes the current working directory to the specified path.
pub fn set_current_dir(path: &str) -> io::Result<()> {
    crate::root::set_current_dir(path)
}

/// Read the entire contents of a file into a bytes vector.
pub fn read(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut bytes = Vec::with_capacity(size as usize);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Read the entire contents of a file into a string.
pub fn read_to_string(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut string = String::with_capacity(size as usize);
    file.read_to_string(&mut string)?;
    Ok(string)
}

/// Write a slice as the entire contents of a file.
pub fn write<C: AsRef<[u8]>>(path: &str, contents: C) -> io::Result<()> {
    File::create(path)?.write_all(contents.as_ref())
}

/// Given a path, query the file system to get information about a file,
/// directory, etc.
pub fn metadata(path: &str) -> io::Result<Metadata> {
    crate::root::lookup(None, path)?.get_attr().map(Metadata)
}

/// Creates a new, empty directory at the provided path.
pub fn create_dir(path: &str) -> io::Result<()> {
    DirBuilder::new().create(path)
}

/// Recursively create a directory and all of its parent components if they
/// are missing.
pub fn create_dir_all(path: &str) -> io::Result<()> {
    DirBuilder::new().recursive(true).create(path)
}

/// Removes an empty directory.
pub fn remove_dir(path: &str) -> io::Result<()> {
    crate::root::remove_dir(None, path)
}

/// Removes a file from the filesystem.
pub fn remove_file(path: &str) -> io::Result<()> {
    crate::root::remove_file(None, path)
}

/// Rename a file or directory to a new name.
/// Delete the original file if `old` already exists.
///
/// This only works then the new path is in the same mounted fs.
pub fn rename(old: &str, new: &str) -> io::Result<()> {
    crate::root::rename(old, new)
}

/// check whether absolute path exists.
pub fn absolute_path_exists(path: &str) -> bool {
    crate::root::lookup(None, path).is_ok()
}

/// Create a symbolic link.
///
/// Creates a symbolic link named `new` which contains the string `old`.
pub fn create_symlink(old: &str, new: &str) -> io::Result<()> {
    crate::root::create_symlink(old, new)
}

/// Read the value of a symbolic link.
///
/// Reads the contents of the symbolic link at `path` and places the result in `buf`.
/// Returns the number of bytes read.
pub fn read_link(path: &str, buf: &mut [u8]) -> io::Result<usize> {
    crate::root::read_link(path, buf)
}

/// Get metadata of a file or directory, without following symbolic links.
///
/// This is similar to `metadata()` but for symbolic links, it returns the metadata
/// of the link itself rather than the file it points to.
pub fn symlink_metadata(path: &str) -> io::Result<Metadata> {
    let node = crate::root::lookup(None, path)?;
    let attr = node.get_attr()?;
    Ok(Metadata(attr))
}

/// Check if a path is a symbolic link.
///
/// Returns `true` if the path refers to a symbolic link, `false` otherwise.
pub fn is_symlink(path: &str) -> io::Result<bool> {
    crate::root::is_symlink(path)
}

/// Set file permissions.
///
/// Changes the permissions of the file at `path` to the specified `mode`.
pub fn set_permissions(path: &str, mode: u16) -> io::Result<()> {
    crate::root::set_perm(path, mode)
}
