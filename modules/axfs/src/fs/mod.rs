cfg_if::cfg_if! {
    if #[cfg(feature = "myfs")] {
        pub mod myfs;
    } else if #[cfg(feature = "lwext4_rs")] {
        pub mod lwext4_rust;
    } else if #[cfg(feature = "fatfs")] {
        pub mod fatfs;
    }

}

#[cfg(feature = "devfs")]
pub use axfs_devfs as devfs;

#[cfg(feature = "ramfs")]
pub mod ramfs;
