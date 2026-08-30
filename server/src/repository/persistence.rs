//! Selectable JSON/MySQL persistence boundary for the world repository.

use super::models::{RepositoryState, StoredState};
use super::mysql::MysqlStore;
use crate::config::ServerConfig;
use std::fs;
use std::io::Write;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING};

#[derive(Debug)]
pub(crate) struct PersistenceBackendError(String);

impl PersistenceBackendError {
    pub(crate) fn new(message: &str) -> Self {
        Self(message.to_owned())
    }
}

impl std::fmt::Display for PersistenceBackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub(super) enum PersistenceBackend {
    Json,
    Mysql(MysqlStore),
}

impl PersistenceBackend {
    pub(super) fn open(
        config: &ServerConfig,
    ) -> Result<(Self, Option<RepositoryState>), PersistenceBackendError> {
        match config.db_driver.trim().to_ascii_lowercase().as_str() {
            "json" => Ok((Self::Json, load_state(config)?)),
            "mysql" => {
                let store = MysqlStore::open(config)?;
                let state = store.load(config)?;
                Ok((Self::Mysql(store), state))
            }
            driver => Err(PersistenceBackendError::new(&format!(
                "unsupported DB_DRIVER `{driver}`; use `json` or `mysql`"
            ))),
        }
    }

    pub(super) fn persist(
        &self,
        state: &RepositoryState,
        config: &ServerConfig,
    ) -> Result<(), PersistenceBackendError> {
        match self {
            Self::Json => persist_json(state, config),
            Self::Mysql(store) => store.persist(state),
        }
    }
}

pub(super) fn load_state(
    config: &ServerConfig,
) -> Result<Option<RepositoryState>, PersistenceBackendError> {
    let Some(path) = config.persistence_path.as_deref() else {
        return Ok(None);
    };
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(PersistenceBackendError::new(
                "the JSON world snapshot could not be read",
            ));
        }
    };
    let stored: StoredState = serde_json::from_slice(&bytes).map_err(|_| {
        PersistenceBackendError::new("the JSON world snapshot contains invalid state JSON")
    })?;
    if stored.storage_version > super::STORAGE_VERSION {
        return Err(PersistenceBackendError::new(
            "the JSON world snapshot was created by a newer server version",
        ));
    }
    Ok(Some(RepositoryState::from_stored(stored, config)))
}

fn persist_json(
    state: &RepositoryState,
    config: &ServerConfig,
) -> Result<(), PersistenceBackendError> {
    let Some(path) = config.persistence_path.as_deref() else {
        return Ok(());
    };
    let data = serde_json::to_vec_pretty(&state.to_stored()).map_err(|_| {
        PersistenceBackendError::new("the JSON world snapshot could not be encoded")
    })?;
    let path = Path::new(path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|_| {
            PersistenceBackendError::new("the JSON world snapshot directory could not be created")
        })?;
    }
    let temporary_path = path.with_extension(format!(
        "{}-{}",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("state"),
        std::process::id()
    ));
    if write_and_sync(&temporary_path, &data).is_err() {
        let _ = fs::remove_file(&temporary_path);
        return Err(PersistenceBackendError::new(
            "the JSON world snapshot could not be written",
        ));
    }
    replace_file(&temporary_path, path).map_err(|_| {
        let _ = fs::remove_file(&temporary_path);
        PersistenceBackendError::new("the JSON world snapshot could not be replaced")
    })
}

pub(super) fn write_and_sync(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    file.sync_all()
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
