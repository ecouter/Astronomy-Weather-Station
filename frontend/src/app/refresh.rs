//! Shared background runtime for all asynchronous work.
//!
//! Previously each job spawned its own OS thread and created a fresh Tokio
//! runtime with `Runtime::new() + block_on`. Some paths even did that *inside*
//! `invoke_from_event_loop`, which blocked the Slint UI thread for the whole
//! fetch and was the main source of visible lags.
//!
//! All background work now goes through the single runtime below. UI property
//! updates always happen on the Slint event loop via
//! `Weak::upgrade_in_event_loop`, never inside a background task.

use std::future::Future;
use std::time::Duration;

use once_cell::sync::Lazy;

/// One multi-threaded runtime shared by the whole application.
pub static RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to start the shared background runtime")
});

/// Spawn a one-shot background job.
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    RUNTIME.spawn(future);
}

/// Spawn a periodic job. The first tick is skipped so the body runs only after
/// `interval` has elapsed (matching the old `interval.tick().await` behaviour).
pub fn start_periodic<F, Fut>(interval: Duration, mut make_future: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.tick().await; // skip the immediate first tick
        loop {
            ticker.tick().await;
            make_future().await;
        }
    });
}
