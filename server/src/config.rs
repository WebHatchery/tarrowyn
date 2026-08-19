use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub world_width: u32,
    pub world_height: u32,
    pub day_length_seconds: f32,
    pub tick_interval: Duration,
    pub world_seconds_per_tick: f32,
    pub session_ttl_seconds: u32,
    pub movement_cooldown_ticks: u64,
    pub chat_max_length: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8787".to_owned(),
            world_width: 18,
            world_height: 11,
            day_length_seconds: 180.0,
            tick_interval: Duration::from_millis(250),
            world_seconds_per_tick: 1.0,
            session_ttl_seconds: 30,
            movement_cooldown_ticks: 1,
            chat_max_length: 160,
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            bind_addr: env_string("TARROWYN_SERVER_ADDR", defaults.bind_addr),
            world_width: env_u32("TARROWYN_WORLD_WIDTH", defaults.world_width),
            world_height: env_u32("TARROWYN_WORLD_HEIGHT", defaults.world_height),
            day_length_seconds: env_f32("TARROWYN_DAY_LENGTH_SECONDS", defaults.day_length_seconds),
            tick_interval: Duration::from_millis(env_u64(
                "TARROWYN_TICK_MS",
                defaults.tick_interval.as_millis() as u64,
            )),
            world_seconds_per_tick: env_f32(
                "TARROWYN_WORLD_SECONDS_PER_TICK",
                defaults.world_seconds_per_tick,
            ),
            session_ttl_seconds: env_u32(
                "TARROWYN_SESSION_TTL_SECONDS",
                defaults.session_ttl_seconds,
            ),
            movement_cooldown_ticks: env_u64(
                "TARROWYN_MOVEMENT_COOLDOWN_TICKS",
                defaults.movement_cooldown_ticks,
            ),
            chat_max_length: env_u64("TARROWYN_CHAT_MAX_LENGTH", defaults.chat_max_length as u64)
                as usize,
        }
    }

    pub fn session_ttl_ticks(&self) -> u64 {
        let tick_seconds = self.tick_interval.as_secs_f32().max(0.001);
        ((self.session_ttl_seconds as f32 / tick_seconds).ceil() as u64).max(1)
    }
}

fn env_string(name: &str, default: String) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_u32(name: &str, default: u32) -> u32 {
    env_u64(name, default as u64) as u32
}

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f32| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
}
