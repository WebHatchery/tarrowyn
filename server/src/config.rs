use std::time::Duration;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub db_driver: String,
    pub db_host: String,
    pub db_port: u16,
    pub db_database: String,
    pub db_username: String,
    pub db_password: String,
    pub world_width: u32,
    pub world_height: u32,
    pub day_length_seconds: f32,
    pub tick_interval: Duration,
    pub world_seconds_per_tick: f32,
    pub session_ttl_seconds: u32,
    pub movement_cooldown_ticks: u64,
    pub combat_action_cooldown_ticks: u64,
    pub chat_max_length: usize,
    pub moderation_cooldown_ticks: u64,
    pub starting_gold: u32,
    pub starting_seeds: u32,
    pub crop_stage_seconds: f32,
    pub trade_expiry_ticks: u64,
    pub claim_reclaim_ticks: u64,
    pub claim_reclaim_grace_ticks: u64,
    pub lease_duration_seconds: u64,
    pub governance_inactivity_ticks: u64,
    pub household_decision_interval_ticks: u64,
    pub expedition_min_food: u32,
    pub expedition_min_tools: u32,
    pub expedition_min_materials: u32,
    pub expedition_min_safety: u32,
    pub persistence_path: Option<String>,
    pub backup_path: Option<String>,
    pub backup_interval_ticks: u64,
    pub production_session_ttl_seconds: u32,
    pub refresh_ttl_seconds: u32,
    pub maintenance_message: Option<String>,
    pub support_operator_accounts: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let content = crate::content::game_config_defaults();
        Self {
            bind_addr: "127.0.0.1:8787".to_owned(),
            db_driver: "json".to_owned(),
            db_host: "localhost".to_owned(),
            db_port: 3306,
            db_database: "tarrowyn".to_owned(),
            db_username: String::new(),
            db_password: String::new(),
            world_width: content.world_width,
            world_height: content.world_height,
            day_length_seconds: content.day_length_seconds,
            tick_interval: Duration::from_millis(250),
            world_seconds_per_tick: 0.25,
            session_ttl_seconds: 30,
            movement_cooldown_ticks: 1,
            combat_action_cooldown_ticks: 1,
            chat_max_length: 160,
            moderation_cooldown_ticks: 20,
            starting_gold: content.starting_gold,
            starting_seeds: content.starting_seeds,
            crop_stage_seconds: 30.0,
            trade_expiry_ticks: 240,
            claim_reclaim_ticks: 480,
            claim_reclaim_grace_ticks: 4,
            lease_duration_seconds: 90 * 24 * 60 * 60,
            governance_inactivity_ticks: 48,
            household_decision_interval_ticks: 4,
            expedition_min_food: 6,
            expedition_min_tools: 3,
            expedition_min_materials: 8,
            expedition_min_safety: 3,
            persistence_path: None,
            backup_path: Some("dist/tarrowyn-server-state.json.backup".to_owned()),
            backup_interval_ticks: 120,
            production_session_ttl_seconds: 900,
            refresh_ttl_seconds: 2_592_000,
            maintenance_message: None,
            support_operator_accounts: Vec::new(),
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let defaults = Self::default();
        let tick_interval = Duration::from_millis(
            env_u64(
                "TARROWYN_TICK_MS",
                defaults.tick_interval.as_millis() as u64,
            )
            .max(1),
        );
        Self {
            bind_addr: env_string("TARROWYN_SERVER_ADDR", defaults.bind_addr),
            db_driver: env_string("DB_DRIVER", defaults.db_driver),
            db_host: env_string("DB_HOST", defaults.db_host),
            db_port: env_u16("DB_PORT", defaults.db_port),
            db_database: env_string("DB_DATABASE", defaults.db_database),
            db_username: env_string("DB_USERNAME", defaults.db_username),
            db_password: env_string("DB_PASSWORD", defaults.db_password),
            world_width: env_u32("TARROWYN_WORLD_WIDTH", defaults.world_width),
            world_height: env_u32("TARROWYN_WORLD_HEIGHT", defaults.world_height),
            day_length_seconds: env_f32("TARROWYN_DAY_LENGTH_SECONDS", defaults.day_length_seconds),
            tick_interval,
            world_seconds_per_tick: env_f32(
                "TARROWYN_WORLD_SECONDS_PER_TICK",
                tick_interval.as_secs_f32().max(0.001),
            ),
            session_ttl_seconds: env_u32(
                "TARROWYN_SESSION_TTL_SECONDS",
                defaults.session_ttl_seconds,
            ),
            movement_cooldown_ticks: env_u64(
                "TARROWYN_MOVEMENT_COOLDOWN_TICKS",
                defaults.movement_cooldown_ticks,
            ),
            combat_action_cooldown_ticks: env_u64(
                "TARROWYN_COMBAT_ACTION_COOLDOWN_TICKS",
                defaults.combat_action_cooldown_ticks,
            ),
            chat_max_length: bounded_usize(
                env_u64("TARROWYN_CHAT_MAX_LENGTH", defaults.chat_max_length as u64),
                defaults.chat_max_length,
            ),
            moderation_cooldown_ticks: env_u64(
                "TARROWYN_MODERATION_COOLDOWN_TICKS",
                defaults.moderation_cooldown_ticks,
            ),
            starting_gold: env_u32("TARROWYN_STARTING_GOLD", defaults.starting_gold),
            starting_seeds: env_u32("TARROWYN_STARTING_SEEDS", defaults.starting_seeds),
            crop_stage_seconds: env_f32("TARROWYN_CROP_STAGE_SECONDS", defaults.crop_stage_seconds),
            trade_expiry_ticks: env_u64("TARROWYN_TRADE_EXPIRY_TICKS", defaults.trade_expiry_ticks),
            claim_reclaim_ticks: env_u64(
                "TARROWYN_CLAIM_RECLAIM_TICKS",
                defaults.claim_reclaim_ticks,
            ),
            claim_reclaim_grace_ticks: env_u64(
                "TARROWYN_CLAIM_RECLAIM_GRACE_TICKS",
                defaults.claim_reclaim_grace_ticks,
            ),
            lease_duration_seconds: env_u64(
                "TARROWYN_LEASE_DURATION_SECONDS",
                defaults.lease_duration_seconds,
            ),
            governance_inactivity_ticks: env_u64(
                "TARROWYN_GOVERNANCE_INACTIVITY_TICKS",
                defaults.governance_inactivity_ticks,
            ),
            household_decision_interval_ticks: env_u64(
                "TARROWYN_HOUSEHOLD_DECISION_INTERVAL_TICKS",
                defaults.household_decision_interval_ticks,
            ),
            expedition_min_food: env_u32(
                "TARROWYN_EXPEDITION_MIN_FOOD",
                defaults.expedition_min_food,
            ),
            expedition_min_tools: env_u32(
                "TARROWYN_EXPEDITION_MIN_TOOLS",
                defaults.expedition_min_tools,
            ),
            expedition_min_materials: env_u32(
                "TARROWYN_EXPEDITION_MIN_MATERIALS",
                defaults.expedition_min_materials,
            ),
            expedition_min_safety: env_u32(
                "TARROWYN_EXPEDITION_MIN_SAFETY",
                defaults.expedition_min_safety,
            ),
            persistence_path: env_string_optional(
                "TARROWYN_STATE_PATH",
                Some("dist/tarrowyn-server-state.json".to_owned()),
            ),
            backup_path: env_string_optional("TARROWYN_BACKUP_PATH", defaults.backup_path),
            backup_interval_ticks: env_u64(
                "TARROWYN_BACKUP_INTERVAL_TICKS",
                defaults.backup_interval_ticks,
            ),
            production_session_ttl_seconds: env_u32(
                "TARROWYN_PRODUCTION_SESSION_TTL_SECONDS",
                defaults.production_session_ttl_seconds,
            ),
            refresh_ttl_seconds: env_u32(
                "TARROWYN_REFRESH_TTL_SECONDS",
                defaults.refresh_ttl_seconds,
            ),
            maintenance_message: env_string_optional("TARROWYN_MAINTENANCE_MESSAGE", None),
            support_operator_accounts: env_list("TARROWYN_SUPPORT_OPERATOR_ACCOUNTS"),
        }
    }

    pub fn session_ttl_ticks(&self) -> u64 {
        let tick_seconds = self.tick_interval.as_secs_f32().max(0.001);
        ((self.session_ttl_seconds as f32 / tick_seconds).ceil() as u64).max(1)
    }

    pub fn production_session_ttl_ticks(&self) -> u64 {
        let tick_seconds = self.tick_interval.as_secs_f32().max(0.001);
        ((self.production_session_ttl_seconds as f32 / tick_seconds).ceil() as u64).max(1)
    }

    pub fn refresh_ttl_ticks(&self) -> u64 {
        let tick_seconds = self.tick_interval.as_secs_f32().max(0.001);
        ((self.refresh_ttl_seconds as f32 / tick_seconds).ceil() as u64).max(1)
    }
}

fn env_string(name: &str, default: String) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
}

fn env_string_optional(name: &str, default: Option<String>) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_u32(name: &str, default: u32) -> u32 {
    bounded_u32(env_u64(name, u64::from(default)), default)
}

fn env_u16(name: &str, default: u16) -> u16 {
    bounded_u16(env_u64(name, u64::from(default)), default)
}

fn bounded_u32(value: u64, default: u32) -> u32 {
    u32::try_from(value).unwrap_or(default)
}

fn bounded_u16(value: u64, default: u16) -> u16 {
    u16::try_from(value).unwrap_or(default)
}

fn bounded_usize(value: u64, default: usize) -> usize {
    usize::try_from(value).unwrap_or(default)
}

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f32| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
}

fn env_list(name: &str) -> Vec<String> {
    std::env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}
