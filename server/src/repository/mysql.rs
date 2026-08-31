//! Transactional MySQL snapshot storage for the native authoritative server.

use super::models::{RepositoryState, StoredState};
use super::persistence::PersistenceBackendError;
use crate::config::ServerConfig;
use mysql::prelude::Queryable;
use mysql::{OptsBuilder, Pool, PoolConstraints, PoolOpts, TxOpts};
use std::sync::Mutex;
use std::time::Duration;

const MIGRATION_VERSION: u32 = 1;
const MIGRATION_LOCK_NAME: &str = "tarrowyn-schema-migration";
const WORLD_AUTHORITY_LOCK_NAME: &str = "tarrowyn-world-authority";
const WORLD_AUTHORITY_LOCK_TIMEOUT_SECONDS: u32 = 5;
const MYSQL_POOL_WAIT_TIMEOUT_SECONDS: u64 = 5;
const MIGRATION_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS tarrowyn_schema_migrations (
    version INT UNSIGNED PRIMARY KEY,
    applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB";
const MIGRATION_SQL: &str = include_str!("../../migrations/0001_initial_world.sql");

#[cfg(test)]
mod tests;

pub(super) struct MysqlStore {
    pool: Pool,
    authority_connection: Mutex<Option<mysql::PooledConn>>,
}

impl MysqlStore {
    pub(super) fn open(config: &ServerConfig) -> Result<Self, PersistenceBackendError> {
        if config.db_database.trim().is_empty() {
            return Err(PersistenceBackendError::new(
                "DB_DATABASE must be non-empty when DB_DRIVER=mysql",
            ));
        }
        if config.db_username.trim().is_empty() {
            return Err(PersistenceBackendError::new(
                "DB_USERNAME must be non-empty when DB_DRIVER=mysql",
            ));
        }
        if config.db_password.trim().is_empty() {
            return Err(PersistenceBackendError::new(
                "DB_PASSWORD must be non-empty when DB_DRIVER=mysql",
            ));
        }
        let options = OptsBuilder::new()
            .ip_or_hostname(Some(config.db_host.as_str()))
            .tcp_port(config.db_port)
            .db_name(Some(config.db_database.as_str()))
            .user(Some(config.db_username.as_str()))
            .pass(Some(config.db_password.as_str()))
            .pool_opts(
                PoolOpts::default().with_constraints(
                    PoolConstraints::new(1, config.mysql_pool_max_connections)
                        .expect("MySQL pool maximum is bounded above its reserved connection"),
                ),
            );
        let pool = Pool::new(options).map_err(|_| {
            PersistenceBackendError::new("the MySQL connection pool could not be created")
        })?;
        let store = Self {
            pool,
            authority_connection: Mutex::new(None),
        };
        store.migrate()?;
        store.acquire_authority_lock()?;
        Ok(store)
    }

    pub(super) fn load(
        &self,
        config: &ServerConfig,
    ) -> Result<Option<RepositoryState>, PersistenceBackendError> {
        let mut connection = self.connection()?;
        let snapshot: Option<(u32, u64, u64, String)> = connection
            .exec_first(
                "SELECT storage_version, world_tick, event_cursor, state_json
                 FROM tarrowyn_world_state WHERE id = 1",
                (),
            )
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL world snapshot could not be read")
            })?;
        let Some((storage_version, world_tick, event_cursor, state_json)) = snapshot else {
            let index_count: Option<u64> = connection
                .exec_first(
                    "SELECT COUNT(*) FROM tarrowyn_identity_index WHERE state_id = 1",
                    (),
                )
                .map_err(|_| {
                    PersistenceBackendError::new("the MySQL identity index could not be read")
                })?;
            if index_count != Some(0) {
                return Err(PersistenceBackendError::new(
                    "the MySQL identity index does not match its world snapshot",
                ));
            }
            return Ok(None);
        };
        let identity_index: Vec<(String, String)> = connection
            .query(
                "SELECT account_id, character_id
                 FROM tarrowyn_identity_index WHERE state_id = 1
                 ORDER BY account_id",
            )
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL identity index could not be read")
            })?;
        let stored: StoredState = serde_json::from_str(&state_json).map_err(|_| {
            PersistenceBackendError::new("the MySQL world snapshot contains invalid state JSON")
        })?;
        if storage_version > super::STORAGE_VERSION
            || stored.storage_version > super::STORAGE_VERSION
        {
            return Err(PersistenceBackendError::new(
                "the MySQL world snapshot was created by a newer server version",
            ));
        }
        if !snapshot_metadata_matches(storage_version, world_tick, event_cursor, &stored) {
            return Err(PersistenceBackendError::new(
                "the MySQL world snapshot metadata does not match its JSON state",
            ));
        }
        if !identity_index_matches(&stored, &identity_index) {
            return Err(PersistenceBackendError::new(
                "the MySQL identity index does not match its world snapshot",
            ));
        }
        Ok(Some(RepositoryState::from_stored(stored, config)))
    }

    pub(super) fn persist(&self, state: &RepositoryState) -> Result<(), PersistenceBackendError> {
        let stored = state.to_stored();
        let state_json = serde_json::to_string(&stored)
            .map_err(|_| PersistenceBackendError::new("the world snapshot could not be encoded"))?;
        let mut connection = self.connection()?;
        let mut transaction = connection
            .start_transaction(TxOpts::default())
            .map_err(|_| PersistenceBackendError::new("the MySQL transaction could not start"))?;
        transaction
            .exec_drop(
                "INSERT INTO tarrowyn_world_state
                    (id, storage_version, world_tick, event_cursor, state_json)
                 VALUES (1, ?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE
                    storage_version = VALUES(storage_version),
                    world_tick = VALUES(world_tick),
                    event_cursor = VALUES(event_cursor),
                    state_json = VALUES(state_json)",
                (
                    stored.storage_version,
                    stored.tick,
                    stored.cursor,
                    state_json,
                ),
            )
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL world snapshot could not be written")
            })?;
        transaction
            .exec_drop("DELETE FROM tarrowyn_identity_index", ())
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL identity index could not be rebuilt")
            })?;
        for identity in state.identities.values() {
            transaction
                .exec_drop(
                    "INSERT INTO tarrowyn_identity_index (account_id, character_id)
                     VALUES (?, ?)",
                    (&identity.account_id, &identity.character_id),
                )
                .map_err(|_| {
                    PersistenceBackendError::new("the MySQL identity index could not be written")
                })?;
        }
        transaction
            .commit()
            .map_err(|_| PersistenceBackendError::new("the MySQL transaction could not commit"))
    }

    fn connection(&self) -> Result<mysql::PooledConn, PersistenceBackendError> {
        self.pool
            .try_get_conn(Duration::from_secs(MYSQL_POOL_WAIT_TIMEOUT_SECONDS))
            .map_err(|_| {
                PersistenceBackendError::new("a MySQL connection could not be checked out")
            })
    }

    fn acquire_authority_lock(&self) -> Result<(), PersistenceBackendError> {
        let mut connection = self.connection()?;
        let lock_acquired: Option<u8> = connection
            .exec_first(
                "SELECT IFNULL(GET_LOCK(?, ?), 0)",
                (
                    WORLD_AUTHORITY_LOCK_NAME,
                    WORLD_AUTHORITY_LOCK_TIMEOUT_SECONDS,
                ),
            )
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL world authority lock could not be acquired")
            })?;
        if lock_acquired != Some(1) {
            return Err(PersistenceBackendError::new(
                "another MySQL world authority is already running",
            ));
        }
        *self
            .authority_connection
            .lock()
            .expect("MySQL authority connection lock poisoned") = Some(connection);
        Ok(())
    }

    fn migrate(&self) -> Result<(), PersistenceBackendError> {
        let mut connection = self.connection()?;
        connection.query_drop(MIGRATION_TABLE_SQL).map_err(|_| {
            PersistenceBackendError::new("the MySQL migration table could not be created")
        })?;
        let lock_acquired: Option<u8> = connection
            .exec_first("SELECT IFNULL(GET_LOCK(?, 30), 0)", (MIGRATION_LOCK_NAME,))
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL migration lock could not be acquired")
            })?;
        if lock_acquired != Some(1) {
            return Err(PersistenceBackendError::new(
                "the MySQL migration lock timed out",
            ));
        }
        let migration_result = (|| {
            let applied_versions: Vec<u32> = connection
                .query("SELECT version FROM tarrowyn_schema_migrations")
                .map_err(|_| {
                    PersistenceBackendError::new("the MySQL migration table could not be read")
                })?;
            if let Some(version) = unsupported_migration_version(&applied_versions) {
                return Err(PersistenceBackendError::new(&format!(
                    "the MySQL schema version {version} is newer than this server"
                )));
            }
            if applied_versions.contains(&MIGRATION_VERSION) {
                return Ok(());
            }
            let mut transaction =
                connection
                    .start_transaction(TxOpts::default())
                    .map_err(|_| {
                        PersistenceBackendError::new(
                            "the MySQL migration transaction could not start",
                        )
                    })?;
            for statement in MIGRATION_SQL
                .split(';')
                .map(str::trim)
                .filter(|statement| !statement.is_empty())
            {
                transaction.query_drop(statement).map_err(|_| {
                    PersistenceBackendError::new("the MySQL schema migration failed")
                })?;
            }
            transaction
                .exec_drop(
                    "INSERT INTO tarrowyn_schema_migrations (version) VALUES (?)",
                    (MIGRATION_VERSION,),
                )
                .map_err(|_| {
                    PersistenceBackendError::new("the MySQL migration record could not be written")
                })?;
            transaction.commit().map_err(|_| {
                PersistenceBackendError::new("the MySQL migration transaction could not commit")
            })
        })();
        let _ = connection.exec_drop("SELECT RELEASE_LOCK(?)", (MIGRATION_LOCK_NAME,));
        migration_result
    }
}

fn unsupported_migration_version(versions: &[u32]) -> Option<u32> {
    versions
        .iter()
        .copied()
        .find(|version| *version > MIGRATION_VERSION)
}

fn snapshot_metadata_matches(
    storage_version: u32,
    world_tick: u64,
    event_cursor: u64,
    stored: &StoredState,
) -> bool {
    storage_version == stored.storage_version
        && world_tick == stored.tick
        && event_cursor == stored.cursor
}

fn identity_index_matches(stored: &StoredState, identity_index: &[(String, String)]) -> bool {
    let mut expected = stored
        .identities
        .values()
        .map(|identity| (identity.account_id.clone(), identity.character_id.clone()))
        .collect::<Vec<_>>();
    expected.sort();
    let mut actual = identity_index.to_vec();
    actual.sort();
    expected == actual
}

impl Drop for MysqlStore {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.authority_connection.lock() {
            if let Some(connection) = guard.as_mut() {
                let _ =
                    connection.exec_drop("SELECT RELEASE_LOCK(?)", (WORLD_AUTHORITY_LOCK_NAME,));
            }
            guard.take();
        }
    }
}
