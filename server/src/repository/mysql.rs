//! Transactional MySQL snapshot storage for the native authoritative server.

use super::models::{RepositoryState, StoredState};
use super::persistence::PersistenceBackendError;
use crate::config::ServerConfig;
use mysql::prelude::Queryable;
use mysql::{OptsBuilder, Pool, TxOpts};

const MIGRATION_VERSION: u32 = 1;
const MIGRATION_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS tarrowyn_schema_migrations (
    version INT UNSIGNED PRIMARY KEY,
    applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB";
const MIGRATION_SQL: &str = include_str!("../../migrations/0001_initial_world.sql");

pub(super) struct MysqlStore {
    pool: Pool,
}

impl MysqlStore {
    pub(super) fn open(config: &ServerConfig) -> Result<Self, PersistenceBackendError> {
        if config.db_database.trim().is_empty() {
            return Err(PersistenceBackendError::new(
                "DB_DATABASE must be non-empty when DB_DRIVER=mysql",
            ));
        }
        let options = OptsBuilder::new()
            .ip_or_hostname(Some(config.db_host.as_str()))
            .tcp_port(config.db_port)
            .db_name(Some(config.db_database.as_str()))
            .user(Some(config.db_username.as_str()))
            .pass(Some(config.db_password.as_str()));
        let pool = Pool::new(options).map_err(|_| {
            PersistenceBackendError::new("the MySQL connection pool could not be created")
        })?;
        let store = Self { pool };
        store.migrate()?;
        Ok(store)
    }

    pub(super) fn load(
        &self,
        config: &ServerConfig,
    ) -> Result<Option<RepositoryState>, PersistenceBackendError> {
        let mut connection = self.connection()?;
        let state_json: Option<String> = connection
            .exec_first(
                "SELECT state_json FROM tarrowyn_world_state WHERE id = 1",
                (),
            )
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL world snapshot could not be read")
            })?;
        let Some(state_json) = state_json else {
            return Ok(None);
        };
        let stored: StoredState = serde_json::from_str(&state_json).map_err(|_| {
            PersistenceBackendError::new("the MySQL world snapshot contains invalid state JSON")
        })?;
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
        self.pool.get_conn().map_err(|_| {
            PersistenceBackendError::new("a MySQL connection could not be checked out")
        })
    }

    fn migrate(&self) -> Result<(), PersistenceBackendError> {
        let mut connection = self.connection()?;
        connection.query_drop(MIGRATION_TABLE_SQL).map_err(|_| {
            PersistenceBackendError::new("the MySQL migration table could not be created")
        })?;
        let applied: Option<u32> = connection
            .exec_first(
                "SELECT version FROM tarrowyn_schema_migrations WHERE version = ?",
                (MIGRATION_VERSION,),
            )
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL migration table could not be read")
            })?;
        if applied.is_some() {
            return Ok(());
        }
        let mut transaction = connection
            .start_transaction(TxOpts::default())
            .map_err(|_| {
                PersistenceBackendError::new("the MySQL migration transaction could not start")
            })?;
        for statement in MIGRATION_SQL
            .split(';')
            .map(str::trim)
            .filter(|statement| !statement.is_empty())
        {
            transaction
                .query_drop(statement)
                .map_err(|_| PersistenceBackendError::new("the MySQL schema migration failed"))?;
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
    }
}
