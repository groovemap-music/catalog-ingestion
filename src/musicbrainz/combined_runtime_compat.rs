//! Temporary coordination retained only for the combined-runtime compatibility baseline.
//!
//! The current two-mode binary asks MusicBrainz to wait while the companion Discogs
//! process reports `running`. This is MusicBrainz orchestration compatibility, not a
//! shared ingestion policy. Provider-owned containers are expected to run independently
//! and concurrently after repository identity cutover, so this entire module is a
//! deliberate removal seam.

use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};

pub(crate) const DISCOGS_POLL_INTERVAL: Duration = Duration::from_secs(3600);
pub(crate) const DISCOGS_HEALTH_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const DISCOGS_MAX_UNREACHABLE_RETRIES: u32 = 10;

// The unreachable case is a current combined-runtime startup race, not a
// provider-owned scheduling contract.
#[cfg(not(test))]
pub(crate) const DISCOGS_UNREACHABLE_BASE_DELAY: Duration = Duration::from_secs(5);
#[cfg(test)]
pub(crate) const DISCOGS_UNREACHABLE_BASE_DELAY: Duration = Duration::from_millis(5);

#[cfg(not(test))]
pub(crate) const DISCOGS_UNREACHABLE_MAX_DELAY: Duration = Duration::from_secs(300);
#[cfg(test)]
pub(crate) const DISCOGS_UNREACHABLE_MAX_DELAY: Duration = Duration::from_millis(50);

/// Escalating backoff for a consecutive-unreachable attempt count.
pub(crate) fn discogs_unreachable_backoff(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(16); // guard against shl overflow
    let multiplier = 1u32.checked_shl(exponent).unwrap_or(u32::MAX);
    DISCOGS_UNREACHABLE_BASE_DELAY.saturating_mul(multiplier).min(DISCOGS_UNREACHABLE_MAX_DELAY)
}

/// Outcome of waiting for the Discogs extractor to go idle.
///
/// A plain `Ok(())` used to mean BOTH "Discogs is idle, go ahead" and "shutdown was
/// requested, stop" — so a SIGTERM arriving during the (multi-hour) wait launched a
/// brand-new MusicBrainz extraction run instead of exiting. (discogsography-l114)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitOutcome {
    /// Discogs is idle (or unreachable / unparseable) — start the MusicBrainz run.
    Proceed,
    /// Shutdown was requested while waiting — do not start any new work.
    Shutdown,
}

/// Sleep for `duration`, returning early as soon as shutdown is requested.
///
/// Returns `true` if shutdown was observed. The wait between Discogs health polls is an
/// hour in production; an uninterruptible sleep there means SIGTERM is not even noticed
/// until long after Docker's stop grace period has elapsed and SIGKILL has landed.
async fn sleep_unless_shutdown(duration: Duration, shutdown_flag: &AtomicBool) -> bool {
    const SLICE: Duration = Duration::from_millis(250);

    let deadline = Instant::now() + duration;
    loop {
        if shutdown_flag.load(Ordering::SeqCst) {
            return true;
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }

        sleep(remaining.min(SLICE)).await;
    }
}

/// Wait until the Discogs extractor is not actively extracting.
pub async fn wait_for_discogs_idle(url: &str, shutdown_flag: &AtomicBool) -> Result<WaitOutcome> {
    wait_for_discogs_idle_with_interval(url, shutdown_flag, DISCOGS_POLL_INTERVAL).await
}

/// Internal implementation with configurable poll interval (for testing). `poll_interval`
/// governs only the "Discogs is busy" (status == "running") wait; an unreachable endpoint
/// always uses the short escalating backoff from `discogs_unreachable_backoff`.
pub async fn wait_for_discogs_idle_with_interval(url: &str, shutdown_flag: &AtomicBool, poll_interval: Duration) -> Result<WaitOutcome> {
    let client = reqwest::Client::builder().timeout(DISCOGS_HEALTH_TIMEOUT).build()?;

    let mut unreachable_count: u32 = 0;

    loop {
        if shutdown_flag.load(Ordering::SeqCst) {
            info!("🛑 Shutdown requested, stopping Discogs health check wait");
            return Ok(WaitOutcome::Shutdown);
        }

        match client.get(url).send().await {
            Ok(response) => {
                unreachable_count = 0;
                match response.json::<serde_json::Value>().await {
                    Ok(body) => {
                        let status = body.get("extraction_status").and_then(|v| v.as_str()).unwrap_or("unknown");

                        if status == "running" {
                            info!("⏳ Discogs extraction in progress, waiting before starting MusicBrainz extraction...");
                        } else {
                            info!("✅ Discogs extractor idle (status: {}), proceeding with MusicBrainz extraction", status);
                            return Ok(WaitOutcome::Proceed);
                        }
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to parse Discogs health response: {}, proceeding", e);
                        return Ok(WaitOutcome::Proceed);
                    }
                }
                if sleep_unless_shutdown(poll_interval, shutdown_flag).await {
                    info!("🛑 Shutdown requested, stopping Discogs health check wait");
                    return Ok(WaitOutcome::Shutdown);
                }
            }
            Err(_) => {
                unreachable_count += 1;
                if unreachable_count >= DISCOGS_MAX_UNREACHABLE_RETRIES {
                    warn!(
                        "⚠️ Discogs health endpoint unreachable after {} attempts, proceeding with MusicBrainz extraction",
                        DISCOGS_MAX_UNREACHABLE_RETRIES
                    );
                    return Ok(WaitOutcome::Proceed);
                }
                let backoff = discogs_unreachable_backoff(unreachable_count);
                warn!(
                    "⚠️ Discogs health endpoint unreachable (attempt {}/{}), retrying in {:?}...",
                    unreachable_count, DISCOGS_MAX_UNREACHABLE_RETRIES, backoff
                );
                if sleep_unless_shutdown(backoff, shutdown_flag).await {
                    info!("🛑 Shutdown requested, stopping Discogs health check wait");
                    return Ok(WaitOutcome::Shutdown);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "combined_runtime_compat_tests.rs"]
mod tests;
