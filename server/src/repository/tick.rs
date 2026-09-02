use super::*;
use std::time::{Duration, Instant};

impl WorldRepository {
    pub fn tick(&self) {
        let started = Instant::now();
        let mut state = self.state.lock().expect("world repository lock poisoned");
        state.tick = state.tick.saturating_add(1);
        let day_length =
            if state.clock.day_length_seconds.is_finite() && state.clock.day_length_seconds > 0.0 {
                state.clock.day_length_seconds
            } else {
                1.0
            };
        let current_seconds = if state.clock.seconds.is_finite() && state.clock.seconds >= 0.0 {
            state.clock.seconds
        } else {
            0.0
        };
        let tick_seconds = if self.config.world_seconds_per_tick.is_finite()
            && self.config.world_seconds_per_tick > 0.0
        {
            self.config.world_seconds_per_tick
        } else {
            0.0
        };
        let elapsed_seconds = (current_seconds + tick_seconds).min(f32::MAX);
        let elapsed_days = (elapsed_seconds / day_length).floor();
        let advanced_days = elapsed_days.min(u32::MAX as f32) as u32;
        if elapsed_days > 0.0 {
            state.clock.seconds = elapsed_seconds % day_length;
            state.clock.day = state.clock.day.saturating_add(advanced_days);
        } else {
            state.clock.seconds = elapsed_seconds;
        }
        if advanced_days > 0 {
            phase4::day_rollover(&mut state, advanced_days);
        }
        world::grow_plots(&mut state, &self.config);
        foundation::recover_resource_nodes(&mut state);
        trades::expire_trades(&mut state);
        phase3::tick(&mut state, &self.config);
        phase4::phase4_tick(&mut state, &self.config);
        let clock = state.clock.clone();
        push_event(&mut state, WorldEvent::Clock(clock));
        expire_sessions(&mut state, &self.config);
        if self.persist(&mut state).is_ok() {
            if let Some(backup_ok) = phase6::scheduled_backup(&mut state, &self.config) {
                *self
                    .backup_failed
                    .lock()
                    .expect("backup status lock poisoned") = !backup_ok;
                if backup_ok {
                    // The backup metadata is part of the authoritative snapshot,
                    // so persist the successful backup marker after the file is
                    // safely replaced.
                    let _ = self.persist(&mut state);
                }
            }
        }
        drop(state);
        self.record_tick_duration(started.elapsed());
    }

    pub fn server_tick(&self) -> u64 {
        self.state
            .lock()
            .expect("world repository lock poisoned")
            .tick
    }

    pub(super) fn persist(
        &self,
        state: &mut RepositoryState,
    ) -> Result<(), super::RepositoryError> {
        match self.storage.persist(state, &self.config) {
            Ok(()) => {
                *self
                    .last_persisted_state
                    .lock()
                    .expect("last persisted state lock poisoned") = state.clone();
                *self
                    .persistence_failed
                    .lock()
                    .expect("persistence status lock poisoned") = false;
                Ok(())
            }
            Err(error) => {
                eprintln!("Tarrowyn persistence write failed: {error}");
                *state = self
                    .last_persisted_state
                    .lock()
                    .expect("last persisted state lock poisoned")
                    .clone();
                *self
                    .persistence_failed
                    .lock()
                    .expect("persistence status lock poisoned") = true;
                Err(super::RepositoryError::persistence_unavailable())
            }
        }
    }

    pub(super) fn expire_and_persist_sessions(
        &self,
        state: &mut RepositoryState,
    ) -> Result<(), super::RepositoryError> {
        if expire_sessions(state, &self.config) {
            self.persist(state)?;
        }
        Ok(())
    }

    pub(super) fn record_tick_duration(&self, elapsed: Duration) {
        let elapsed_ms = elapsed.as_millis().min(u128::from(u32::MAX)) as u32;
        let budget_ms = self
            .config
            .tick_interval
            .as_millis()
            .max(1)
            .min(u128::from(u32::MAX)) as u32;
        let mut telemetry = self
            .tick_telemetry
            .lock()
            .expect("tick telemetry lock poisoned");
        telemetry.last_tick_ms = elapsed_ms;
        telemetry.average_tick_ms = if telemetry.average_tick_ms == 0 {
            elapsed_ms
        } else {
            (u64::from(telemetry.average_tick_ms)
                .saturating_mul(7)
                .saturating_add(u64::from(elapsed_ms))
                / 8) as u32
        };
        telemetry.last_tick_drift = elapsed_ms > budget_ms;
        if telemetry.last_tick_drift {
            telemetry.tick_drift_count = telemetry.tick_drift_count.saturating_add(1);
        }
    }
}
