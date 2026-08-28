use super::super::models::RepositoryState;
use super::super::persistence::replace_file;
use crate::config::ServerConfig;
use std::fs;
use std::path::Path;

pub(super) fn write(state: &mut RepositoryState, config: &ServerConfig) {
    let Some(path) = config
        .backup_path
        .as_deref()
        .or(config.persistence_path.as_deref())
    else {
        return;
    };
    let backup_path = if config.backup_path.is_some() {
        path.to_owned()
    } else {
        format!("{path}.backup")
    };
    let Ok(data) = serde_json::to_vec_pretty(&state.to_stored()) else {
        return;
    };
    let path = Path::new(&backup_path);
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let temporary_path = path.with_extension(format!(
        "{}-{}",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("backup"),
        std::process::id()
    ));
    if fs::write(&temporary_path, data).is_ok() && replace_file(&temporary_path, path).is_ok() {
        state.phase6.last_backup_tick = Some(state.tick);
        state.phase6.last_backup_path = Some(backup_path);
    } else {
        let _ = fs::remove_file(temporary_path);
    }
}
