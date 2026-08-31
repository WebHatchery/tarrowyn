use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::thread;

pub(super) const MIN_REQUEST_WORKERS: usize = 4;
pub(super) const MAX_REQUEST_WORKERS: usize = 32;
pub(super) const REQUEST_QUEUE_CAPACITY: usize = 128;

#[derive(Clone, Copy)]
pub(super) struct RequestPoolTelemetrySnapshot {
    pub(super) active_requests: u32,
    pub(super) queue_depth: u32,
    pub(super) queue_peak: u32,
    pub(super) queue_full_events: u64,
}

#[derive(Default)]
pub(super) struct RequestPoolTelemetry {
    active_requests: AtomicU32,
    queue_depth: AtomicU32,
    queue_peak: AtomicU32,
    queue_full_events: AtomicU64,
}

impl RequestPoolTelemetry {
    pub(super) fn record_enqueue(&self) {
        let depth = self.queue_depth.fetch_add(1, Ordering::Relaxed) + 1;
        let mut peak = self.queue_peak.load(Ordering::Relaxed);
        while depth > peak {
            match self.queue_peak.compare_exchange_weak(
                peak,
                depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(current) => peak = current,
            }
        }
    }

    pub(super) fn record_dequeue(&self) {
        self.queue_depth.fetch_sub(1, Ordering::Relaxed);
    }

    pub(super) fn record_queue_full(&self) {
        self.queue_full_events.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn record_request_start(&self) {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn record_request_finish(&self) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    pub(super) fn snapshot(&self) -> RequestPoolTelemetrySnapshot {
        RequestPoolTelemetrySnapshot {
            active_requests: self.active_requests.load(Ordering::Relaxed),
            queue_depth: self.queue_depth.load(Ordering::Relaxed),
            queue_peak: self.queue_peak.load(Ordering::Relaxed),
            queue_full_events: self.queue_full_events.load(Ordering::Relaxed),
        }
    }
}

pub(super) fn request_worker_count() -> usize {
    thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(8)
        .clamp(MIN_REQUEST_WORKERS, MAX_REQUEST_WORKERS)
}
