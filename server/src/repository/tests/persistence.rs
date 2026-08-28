use super::super::WorldRepository;
use crate::config::ServerConfig;

#[test]
fn persistence_backend_rejects_unknown_driver_before_world_start() {
    let config = ServerConfig {
        db_driver: "sqlite".to_owned(),
        ..ServerConfig::default()
    };
    let error = WorldRepository::try_new(config).err().unwrap();
    assert!(error.contains("use `json` or `mysql`"));
}

#[test]
fn mysql_backend_requires_a_database_name_before_connecting() {
    let config = ServerConfig {
        db_driver: "mysql".to_owned(),
        db_database: String::new(),
        ..ServerConfig::default()
    };
    let error = WorldRepository::try_new(config).err().unwrap();
    assert!(error.contains("DB_DATABASE must be non-empty"));
}

#[test]
fn corrupt_json_snapshot_fails_closed_without_overwriting_the_file() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-corrupt-state-{}.json",
        std::process::id()
    ));
    let contents = b"{ this is not a Tarrowyn snapshot }";
    std::fs::write(&path, contents).unwrap();
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };

    let error = WorldRepository::try_new(config).err().unwrap();
    assert!(error.contains("invalid state JSON"));
    assert_eq!(std::fs::read(&path).unwrap(), contents);
    let _ = std::fs::remove_file(path);
}
