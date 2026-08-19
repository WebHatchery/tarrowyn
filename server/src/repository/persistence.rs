//! Atomic state-file loading and replacement for the world repository.

use super::models::{RepositoryState, StoredState};
use crate::config::ServerConfig;
use std::fs;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING};

pub(super) fn load_state(config: &ServerConfig) -> Option<RepositoryState> {
    let bytes = fs::read(config.persistence_path.as_deref()?).ok()?;
    let stored: StoredState = serde_json::from_slice(&bytes).ok()?;
    Some(RepositoryState::from_stored(stored, config))
}

#[cfg(not(windows))]
pub(super) fn replace_file(temporary_path: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(temporary_path, path)
}

#[cfg(windows)]
pub(super) fn replace_file(temporary_path: &Path, path: &Path) -> std::io::Result<()> {
    let temporary_path: Vec<u16> = temporary_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            temporary_path.as_ptr(),
            path.as_ptr(),
            MOVEFILE_REPLACE_EXISTING,
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
