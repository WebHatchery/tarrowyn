use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tiny_http::Request;

pub(super) const GUEST_SESSION_RATE_WINDOW: Duration = Duration::from_secs(60);
pub(super) const GUEST_SESSION_BURST_LIMIT: u16 = 32;
pub(super) const MAX_TRACKED_GUEST_SOURCES: usize = 4096;

pub(super) struct GuestSessionRateLimiter {
    windows: HashMap<IpAddr, GuestSessionRateWindow>,
    burst_limit: u16,
}

struct GuestSessionRateWindow {
    started_at: Instant,
    attempts: u16,
}

impl GuestSessionRateLimiter {
    pub(super) fn new(burst_limit: u16) -> Self {
        Self {
            windows: HashMap::new(),
            burst_limit: burst_limit.max(1),
        }
    }

    pub(super) fn allow(&mut self, request: &Request) -> bool {
        self.allow_ip(
            request.remote_addr().map(|address| address.ip()),
            Instant::now(),
        )
    }

    pub(super) fn allow_ip(&mut self, source: Option<IpAddr>, now: Instant) -> bool {
        let Some(source) = source else {
            return true;
        };
        self.windows.retain(|_, window| {
            now.checked_duration_since(window.started_at)
                .unwrap_or_default()
                < GUEST_SESSION_RATE_WINDOW
        });
        if !self.windows.contains_key(&source) && self.windows.len() >= MAX_TRACKED_GUEST_SOURCES {
            let oldest_source = self
                .windows
                .iter()
                .min_by_key(|(_, window)| window.started_at)
                .map(|(source, _)| *source);
            if let Some(oldest_source) = oldest_source {
                self.windows.remove(&oldest_source);
            }
        }
        let window = self
            .windows
            .entry(source)
            .or_insert(GuestSessionRateWindow {
                started_at: now,
                attempts: 0,
            });
        if now
            .checked_duration_since(window.started_at)
            .unwrap_or_default()
            >= GUEST_SESSION_RATE_WINDOW
        {
            window.started_at = now;
            window.attempts = 0;
        }
        if window.attempts >= self.burst_limit {
            return false;
        }
        window.attempts += 1;
        true
    }

    #[cfg(test)]
    pub(super) fn tracked_source_count(&self) -> usize {
        self.windows.len()
    }
}

impl Default for GuestSessionRateLimiter {
    fn default() -> Self {
        Self::new(GUEST_SESSION_BURST_LIMIT)
    }
}
