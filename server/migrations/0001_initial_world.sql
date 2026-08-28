CREATE TABLE IF NOT EXISTS tarrowyn_schema_migrations (
    version INT UNSIGNED NOT NULL PRIMARY KEY,
    applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS tarrowyn_world_state (
    id TINYINT UNSIGNED NOT NULL PRIMARY KEY,
    storage_version INT UNSIGNED NOT NULL,
    world_tick BIGINT UNSIGNED NOT NULL,
    event_cursor BIGINT UNSIGNED NOT NULL,
    state_json JSON NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    CHECK (id = 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS tarrowyn_identity_index (
    account_id VARCHAR(128) NOT NULL PRIMARY KEY,
    character_id VARCHAR(128) NOT NULL,
    state_id TINYINT UNSIGNED NOT NULL DEFAULT 1,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uq_tarrowyn_identity_character (character_id),
    KEY idx_tarrowyn_identity_state (state_id),
    CONSTRAINT fk_tarrowyn_identity_state
        FOREIGN KEY (state_id) REFERENCES tarrowyn_world_state (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
