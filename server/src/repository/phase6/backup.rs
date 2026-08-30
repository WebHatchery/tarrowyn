use super::super::models::RepositoryState;
use super::super::persistence::replace_file;
use crate::config::ServerConfig;
use std::fs;
use std::io::Write;
use std::path::Path;

pub(super) fn write(state: &mut RepositoryState, config: &ServerConfig) -> bool {
    let Some(path) = config
        .backup_path
        .as_deref()
        .or(config.persistence_path.as_deref())
    else {
        return true;
    };
    let backup_path = if config.backup_path.is_some() {
        path.to_owned()
    } else {
        format!("{path}.backup")
    };
    let Ok(data) = serde_json::to_vec_pretty(&state.to_stored()) else {
        eprintln!("Tarrowyn backup write failed: the snapshot could not be encoded");
        return false;
    };
    let path = Path::new(&backup_path);
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            eprintln!("Tarrowyn backup write failed: could not create the backup directory");
            return false;
        }
    }
    let temporary_path = path.with_extension(format!(
        "{}-{}",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("backup"),
        std::process::id()
    ));
    if write_and_sync(&temporary_path, &data).is_ok() && replace_file(&temporary_path, path).is_ok()
    {
        state.phase6.last_backup_tick = Some(state.tick);
        state.phase6.last_backup_path = Some(backup_path);
        true
    } else {
        let _ = fs::remove_file(temporary_path);
        eprintln!("Tarrowyn backup write failed: could not replace the backup snapshot");
        false
    }
}

fn write_and_sync(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    file.sync_all()
}
