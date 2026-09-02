//! Characterization of the removable combined-runtime coordination seam.

#[test]
fn coordination_is_musicbrainz_owned_compatibility() {
    assert!(module_path!().contains("musicbrainz::combined_runtime_compat"));
}

mod wait_for_discogs_idle_tests {
    use super::super::{WaitOutcome, wait_for_discogs_idle, wait_for_discogs_idle_with_interval};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_proceeds_when_idle() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"extraction_status": "idle"}"#)
            .create_async()
            .await;

        let shutdown = AtomicBool::new(false);
        let url = format!("{}/health", server.url());
        let result = wait_for_discogs_idle(&url, &shutdown).await;

        assert_eq!(result.unwrap(), WaitOutcome::Proceed);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_proceeds_when_completed() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"extraction_status": "completed"}"#)
            .create_async()
            .await;

        let shutdown = AtomicBool::new(false);
        let url = format!("{}/health", server.url());
        let result = wait_for_discogs_idle(&url, &shutdown).await;

        assert_eq!(result.unwrap(), WaitOutcome::Proceed);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_proceeds_when_failed() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"extraction_status": "failed"}"#)
            .create_async()
            .await;

        let shutdown = AtomicBool::new(false);
        let url = format!("{}/health", server.url());
        let result = wait_for_discogs_idle(&url, &shutdown).await;

        assert_eq!(result.unwrap(), WaitOutcome::Proceed);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_proceeds_when_waiting() {
        // "waiting" is the dominant observable state between periodic runs — the
        // MusicBrainz extractor must proceed immediately instead of polling.
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"extraction_status": "waiting"}"#)
            .create_async()
            .await;

        let shutdown = AtomicBool::new(false);
        let url = format!("{}/health", server.url());
        let result = wait_for_discogs_idle(&url, &shutdown).await;

        assert_eq!(result.unwrap(), WaitOutcome::Proceed);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_waits_then_proceeds_when_running_then_idle() {
        let mut server = mockito::Server::new_async().await;
        let _mock_running = server
            .mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"extraction_status": "running"}"#)
            .expect(1)
            .create_async()
            .await;
        let _mock_idle = server
            .mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"extraction_status": "idle"}"#)
            .expect(1)
            .create_async()
            .await;

        let shutdown = AtomicBool::new(false);
        let url = format!("{}/health", server.url());
        let result = wait_for_discogs_idle_with_interval(&url, &shutdown, Duration::from_millis(10)).await;

        assert_eq!(result.unwrap(), WaitOutcome::Proceed);
    }

    #[tokio::test]
    async fn test_proceeds_after_max_unreachable_retries() {
        // Use a port that nothing listens on
        let url = "http://127.0.0.1:19999/health";
        let shutdown = AtomicBool::new(false);
        let result = wait_for_discogs_idle_with_interval(url, &shutdown, Duration::from_millis(10)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_respects_shutdown_signal() {
        let shutdown = AtomicBool::new(true);
        // Use unreachable port — should return immediately due to shutdown flag
        let url = "http://127.0.0.1:19999/health";
        let result = wait_for_discogs_idle_with_interval(url, &shutdown, Duration::from_millis(10)).await;

        // discogsography-l114: a shutdown must be distinguishable from "Discogs is idle",
        // otherwise the caller falls through and starts a whole new extraction run.
        assert_eq!(result.unwrap(), WaitOutcome::Shutdown);
    }

    #[tokio::test]
    async fn test_shutdown_while_discogs_busy() {
        // Discogs keeps reporting "running" (the multi-hour park). SIGTERM arrives mid-wait:
        // the wait must end promptly and report Shutdown, not Proceed.
        let mut server = mockito::Server::new_async().await;
        let _mock_running = server
            .mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"extraction_status": "running"}"#)
            .create_async()
            .await;

        let shutdown = Arc::new(AtomicBool::new(false));
        let url = format!("{}/health", server.url());

        let flag = shutdown.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag.store(true, Ordering::SeqCst);
        });

        // A poll interval far longer than the test timeout proves the sleep is interruptible.
        let outcome = tokio::time::timeout(Duration::from_secs(10), wait_for_discogs_idle_with_interval(&url, &shutdown, Duration::from_secs(3600)))
            .await
            .expect("wait must observe shutdown instead of sleeping out the poll interval");

        assert_eq!(outcome.unwrap(), WaitOutcome::Shutdown);
    }

    #[tokio::test]
    async fn test_shutdown_during_unreachable_retry_backoff() {
        // Same guarantee on the unreachable-endpoint retry path.
        let shutdown = Arc::new(AtomicBool::new(false));
        let flag = shutdown.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            flag.store(true, Ordering::SeqCst);
        });

        let url = "http://127.0.0.1:19999/health";
        let outcome = tokio::time::timeout(Duration::from_secs(10), wait_for_discogs_idle_with_interval(url, &shutdown, Duration::from_secs(3600)))
            .await
            .expect("wait must return once shutdown is observed");

        // Either the retries were exhausted first (Proceed) or shutdown won — but it must
        // never hang, and once the flag is set the loop must stop waiting.
        assert!(matches!(outcome.unwrap(), WaitOutcome::Shutdown | WaitOutcome::Proceed));
    }

    #[tokio::test]
    async fn test_unreachable_endpoint_uses_short_backoff_not_poll_interval() {
        // discogsography-i7sa regression: an unreachable Discogs health endpoint (a
        // startup-ordering race, not "Discogs is busy") must retry on the short
        // escalating backoff, NOT the hourly poll_interval — otherwise a single
        // unlucky restart-timing race costs hours instead of minutes. Pass a
        // deliberately huge poll_interval (as production does — DISCOGS_POLL_INTERVAL
        // is 3600s) and prove all DISCOGS_MAX_UNREACHABLE_RETRIES attempts complete
        // in well under one poll_interval's worth of real time.
        let url = "http://127.0.0.1:19999/health";
        let shutdown = AtomicBool::new(false);
        let huge_poll_interval = Duration::from_secs(3600);

        let start = tokio::time::Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(10), wait_for_discogs_idle_with_interval(url, &shutdown, huge_poll_interval)).await;
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "must give up after DISCOGS_MAX_UNREACHABLE_RETRIES, not hang for poll_interval-per-attempt");
        assert_eq!(result.unwrap().unwrap(), WaitOutcome::Proceed);
        assert!(elapsed < Duration::from_secs(5), "unreachable retries must not wait a full poll_interval per attempt: took {:?}", elapsed);
    }
}

mod discogs_unreachable_backoff_tests {
    use super::super::{DISCOGS_UNREACHABLE_MAX_DELAY, discogs_unreachable_backoff};

    #[test]
    fn test_escalates_and_caps() {
        // discogsography-i7sa: escalating (each attempt waits longer than the last)
        // and bounded (never exceeds the cap, so retries stay "minutes, not hours").
        let mut previous = discogs_unreachable_backoff(1);
        for attempt in 2..=10u32 {
            let backoff = discogs_unreachable_backoff(attempt);
            assert!(backoff >= previous, "attempt {} backoff must not shrink vs attempt {}", attempt, attempt - 1);
            assert!(backoff <= DISCOGS_UNREACHABLE_MAX_DELAY, "attempt {} backoff must never exceed the cap", attempt);
            previous = backoff;
        }
    }

    #[test]
    fn test_zero_and_overflow_safe_attempts() {
        // attempt=0 and very large attempts must not panic (shift-overflow guard).
        let _ = discogs_unreachable_backoff(0);
        let capped = discogs_unreachable_backoff(u32::MAX);
        assert_eq!(capped, DISCOGS_UNREACHABLE_MAX_DELAY);
    }
}
