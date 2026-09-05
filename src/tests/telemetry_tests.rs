//! Telemetry tests driven through the SDK's in-memory metric exporter.
//!
//! Every assertion is a *presence* assertion (this instrument name exists, carrying at least
//! these attribute keys). The whole test binary shares one meter provider — the instruments
//! bind exactly once per process — so other tests contribute data points to the same
//! instruments. Presence assertions are stable under that; exact-value assertions would not
//! be.

use super::*;
use crate::message_queue::MockMessagePublisher;
use crate::runtime::{BatcherConfig, ExtractorState, message_batcher, message_publisher};
use crate::state_marker::StateMarker;
use crate::types::{DataMessage, DataType};
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
use serial_test::serial;
use std::collections::BTreeSet;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Export everything recorded so far and return the flat list of (name, attribute keys)
/// observations, one entry per data point.
fn flush_observations() -> Vec<(String, BTreeSet<String>)> {
    let harness = &*crate::telemetry::test_harness::HARNESS;
    harness.provider.force_flush().expect("force_flush should succeed for the in-memory exporter");

    let mut observations = Vec::new();
    for resource_metrics in harness.exporter.get_finished_metrics().expect("in-memory exporter should hand back its batches") {
        for scope in resource_metrics.scope_metrics() {
            for metric in scope.metrics() {
                let name = metric.name().to_string();
                for keys in attribute_key_sets(metric.data()) {
                    observations.push((name.clone(), keys));
                }
            }
        }
    }
    observations
}

/// The attribute key set of every data point in one aggregated metric.
fn attribute_key_sets(data: &AggregatedMetrics) -> Vec<BTreeSet<String>> {
    fn from_u64(data: &MetricData<u64>) -> Vec<BTreeSet<String>> {
        match data {
            MetricData::Sum(sum) => sum.data_points().map(|dp| dp.attributes().map(|kv| kv.key.to_string()).collect()).collect(),
            MetricData::Gauge(gauge) => gauge.data_points().map(|dp| dp.attributes().map(|kv| kv.key.to_string()).collect()).collect(),
            MetricData::Histogram(h) => h.data_points().map(|dp| dp.attributes().map(|kv| kv.key.to_string()).collect()).collect(),
            MetricData::ExponentialHistogram(h) => h.data_points().map(|dp| dp.attributes().map(|kv| kv.key.to_string()).collect()).collect(),
        }
    }
    fn from_f64(data: &MetricData<f64>) -> Vec<BTreeSet<String>> {
        match data {
            MetricData::Sum(sum) => sum.data_points().map(|dp| dp.attributes().map(|kv| kv.key.to_string()).collect()).collect(),
            MetricData::Gauge(gauge) => gauge.data_points().map(|dp| dp.attributes().map(|kv| kv.key.to_string()).collect()).collect(),
            MetricData::Histogram(h) => h.data_points().map(|dp| dp.attributes().map(|kv| kv.key.to_string()).collect()).collect(),
            MetricData::ExponentialHistogram(h) => h.data_points().map(|dp| dp.attributes().map(|kv| kv.key.to_string()).collect()).collect(),
        }
    }
    match data {
        AggregatedMetrics::U64(d) => from_u64(d),
        AggregatedMetrics::F64(d) => from_f64(d),
        AggregatedMetrics::I64(_) => Vec::new(),
    }
}

/// Assert that `name` was exported with at least one data point carrying exactly `expected`
/// as its attribute keys.
fn assert_instrument(observations: &[(String, BTreeSet<String>)], name: &str, expected: &[&str]) {
    let expected: BTreeSet<String> = expected.iter().map(|k| (*k).to_string()).collect();
    let seen: Vec<&BTreeSet<String>> = observations.iter().filter(|(n, _)| n == name).map(|(_, keys)| keys).collect();

    assert!(!seen.is_empty(), "instrument {name} was never exported; exported: {:?}", observations.iter().map(|(n, _)| n).collect::<BTreeSet<_>>());
    assert!(seen.iter().any(|keys| **keys == expected), "instrument {name} attribute keys mismatch: expected {expected:?}, saw {seen:?}");
}

fn sample_message(id: &str) -> DataMessage {
    DataMessage { id: id.to_string(), sha256: String::new(), data: serde_json::json!({"name": id}), raw_xml: None }
}

fn batcher_config(batch_size: usize, temp: &tempfile::TempDir) -> BatcherConfig {
    BatcherConfig {
        batch_size,
        data_type: DataType::Artists,
        state: Arc::new(RwLock::new(ExtractorState::default())),
        state_marker: Arc::new(tokio::sync::Mutex::new(StateMarker::new("20260101".to_string()))),
        marker_path: temp.path().join("marker.json"),
        file_name: "discogs_20260101_artists.xml.gz".to_string(),
        state_save_interval: 5000,
    }
}

/// The parse/batch path: records flowing through `message_batcher` land on
/// `groovemap.extraction.records` with the conventional `{source, entity}` attribute set.
#[tokio::test]
async fn parse_path_records_extraction_records() {
    set_source(SOURCE_DISCOGS);

    let temp = tempfile::tempdir().expect("tempdir");
    let (record_tx, record_rx) = mpsc::channel::<DataMessage>(16);
    let (batch_tx, mut batch_rx) = mpsc::channel::<Vec<DataMessage>>(16);

    let drain = tokio::spawn(async move { while batch_rx.recv().await.is_some() {} });

    for i in 0..4 {
        record_tx.send(sample_message(&format!("a{i}"))).await.expect("send record");
    }
    drop(record_tx);

    message_batcher(record_rx, batch_tx, batcher_config(2, &temp)).await.expect("batcher should drain cleanly");
    drain.await.expect("drain task");

    let observations = flush_observations();
    assert_instrument(&observations, METRIC_EXTRACTION_RECORDS, &[ATTR_SOURCE, ATTR_ENTITY]);
}

/// The publish path: a failing publisher lands on `groovemap.extraction.errors` with
/// `{source, stage}`, and the stage value is `publish`.
#[tokio::test]
async fn publish_path_records_publish_stage_errors() {
    set_source(SOURCE_DISCOGS);

    let mut publisher = MockMessagePublisher::new();
    publisher.expect_publish_batch().returning(|_, _| Err(anyhow::anyhow!("broker refused the batch")));

    let (batch_tx, batch_rx) = mpsc::channel::<Vec<DataMessage>>(4);
    batch_tx.send(vec![sample_message("a0")]).await.expect("send batch");
    drop(batch_tx);

    let result = message_publisher(batch_rx, Arc::new(publisher), DataType::Artists, Arc::new(RwLock::new(ExtractorState::default()))).await;
    assert!(result.is_err(), "a failing publisher must propagate the error");

    let observations = flush_observations();
    assert_instrument(&observations, METRIC_EXTRACTION_ERRORS, &[ATTR_SOURCE, ATTR_STAGE]);
}

/// Every instrument in the GrooveMap catalog is exported under its conventional name with
/// its conventional attribute set.
#[tokio::test]
async fn every_instrument_uses_its_conventional_name_and_attribute_keys() {
    set_source(SOURCE_DISCOGS);

    record_records("artists", 3);
    record_file_outcome(FileOutcome::Completed);
    record_file_outcome(FileOutcome::Skipped);
    record_file_outcome(FileOutcome::Failed);
    record_file_progress("artists", 0.5);
    record_download_bytes(2048);
    record_publish_confirm(Duration::from_millis(12));
    record_error(Stage::Download);
    record_error(Stage::Parse);
    record_error(Stage::Publish);
    record_messages_sent("groovemap-discogs-artists", 1);
    record_reconnect(MESSAGING_SYSTEM_RABBITMQ);

    let observations = flush_observations();

    assert_instrument(&observations, METRIC_EXTRACTION_RECORDS, &[ATTR_SOURCE, ATTR_ENTITY]);
    assert_instrument(&observations, METRIC_EXTRACTION_FILES, &[ATTR_SOURCE, ATTR_OUTCOME]);
    assert_instrument(&observations, METRIC_EXTRACTION_FILE_PROGRESS, &[ATTR_SOURCE, ATTR_ENTITY]);
    assert_instrument(&observations, METRIC_EXTRACTION_DOWNLOAD_BYTES, &[ATTR_SOURCE]);
    assert_instrument(&observations, METRIC_EXTRACTION_PUBLISH_CONFIRM_DURATION, &[ATTR_SOURCE]);
    assert_instrument(&observations, METRIC_EXTRACTION_ERRORS, &[ATTR_SOURCE, ATTR_STAGE]);
    assert_instrument(&observations, METRIC_MESSAGING_SENT_MESSAGES, &[ATTR_MESSAGING_SYSTEM, ATTR_MESSAGING_DESTINATION_NAME]);
    assert_instrument(&observations, METRIC_PIPELINE_RECONNECTS, &[ATTR_SYSTEM]);
}

/// The publish-confirm histogram carries the `s` unit and second-scale buckets, not the
/// SDK's millisecond-scale defaults.
#[tokio::test]
async fn publish_confirm_histogram_uses_second_scale_buckets() {
    record_publish_confirm(Duration::from_millis(7));

    let harness = &*crate::telemetry::test_harness::HARNESS;
    harness.provider.force_flush().expect("force_flush");

    let mut checked = false;
    for resource_metrics in harness.exporter.get_finished_metrics().expect("finished metrics") {
        for scope in resource_metrics.scope_metrics() {
            for metric in scope.metrics() {
                if metric.name() != METRIC_EXTRACTION_PUBLISH_CONFIRM_DURATION {
                    continue;
                }
                assert_eq!(metric.unit(), "s", "confirm duration must be reported in seconds");
                if let AggregatedMetrics::F64(MetricData::Histogram(histogram)) = metric.data() {
                    for point in histogram.data_points() {
                        let bounds: Vec<f64> = point.bounds().collect();
                        assert_eq!(bounds, CONFIRM_DURATION_BOUNDARIES.to_vec(), "histogram must use the second-scale view boundaries");
                        checked = true;
                    }
                }
            }
        }
    }
    assert!(checked, "the publish-confirm histogram was never exported");
}

/// The exporter is cumulative, which is what the Prometheus remote-write path expects.
#[tokio::test]
async fn counters_are_cumulative() {
    record_download_bytes(1);

    let harness = &*crate::telemetry::test_harness::HARNESS;
    harness.provider.force_flush().expect("force_flush");

    let mut checked = false;
    for resource_metrics in harness.exporter.get_finished_metrics().expect("finished metrics") {
        for scope in resource_metrics.scope_metrics() {
            for metric in scope.metrics() {
                if metric.name() != METRIC_EXTRACTION_DOWNLOAD_BYTES {
                    continue;
                }
                assert_eq!(metric.unit(), "By", "download bytes must be reported in bytes");
                if let AggregatedMetrics::U64(MetricData::Sum(sum)) = metric.data() {
                    assert_eq!(sum.temporality(), opentelemetry_sdk::metrics::Temporality::Cumulative);
                    assert!(sum.is_monotonic(), "a byte counter must be monotonic");
                    checked = true;
                }
            }
        }
    }
    assert!(checked, "the download-bytes counter was never exported");
}

/// A ratio outside 0..1, or a non-finite one, is clamped rather than exported as-is.
#[test]
fn file_progress_is_clamped_to_a_ratio() {
    record_file_progress("artists", 4.2);
    record_file_progress("artists", -1.0);
    record_file_progress("artists", f64::NAN);

    let harness = &*crate::telemetry::test_harness::HARNESS;
    harness.provider.force_flush().expect("force_flush");

    for resource_metrics in harness.exporter.get_finished_metrics().expect("finished metrics") {
        for scope in resource_metrics.scope_metrics() {
            for metric in scope.metrics() {
                if metric.name() != METRIC_EXTRACTION_FILE_PROGRESS {
                    continue;
                }
                if let AggregatedMetrics::F64(MetricData::Gauge(gauge)) = metric.data() {
                    for point in gauge.data_points() {
                        let value = point.value();
                        assert!((0.0..=1.0).contains(&value), "file progress must stay within 0..1, saw {value}");
                    }
                }
            }
        }
    }
}

/// Consuming a `ProgressReader` reports the file-progress gauge, ending at 1.0.
#[test]
fn progress_reader_reports_completion() {
    let payload = vec![b'x'; 4096];
    let total = payload.len() as u64;
    let mut reader = progress_reader(std::io::Cursor::new(payload), total, "artists");

    let mut sink = Vec::new();
    std::io::copy(&mut reader, &mut sink).expect("copy through the progress reader");
    assert_eq!(sink.len(), total as usize);
    assert!((reader.ratio() - 1.0).abs() < f64::EPSILON, "a fully consumed reader must sit at ratio 1.0");
}

/// A zero-length file reports 0.0 rather than dividing by zero.
#[test]
fn progress_reader_handles_an_empty_file() {
    let mut reader = progress_reader(std::io::Cursor::new(Vec::new()), 0, "artists");
    let mut sink = Vec::new();
    std::io::copy(&mut reader, &mut sink).expect("copy through the progress reader");
    assert_eq!(reader.ratio(), 0.0);
}

/// With no OTLP endpoint configured the bootstrap is a no-op that returns `None` instead of
/// panicking or failing startup.
#[tokio::test]
#[serial]
async fn init_metrics_is_a_no_op_without_an_endpoint() {
    unsafe {
        std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        std::env::remove_var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT");
        std::env::remove_var("OTEL_METRICS_EXPORTER");
    }
    assert!(init_metrics(DEFAULT_SERVICE_NAME).is_none());
}

/// An empty endpoint value is treated as unset, not as a malformed URL.
#[tokio::test]
#[serial]
async fn init_metrics_treats_an_empty_endpoint_as_unset() {
    unsafe {
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "   ");
        std::env::remove_var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT");
        std::env::remove_var("OTEL_METRICS_EXPORTER");
    }
    let provider = init_metrics(DEFAULT_SERVICE_NAME);
    unsafe {
        std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    }
    assert!(provider.is_none());
}

/// `OTEL_METRICS_EXPORTER=none` disables export even when an endpoint is configured.
#[tokio::test]
#[serial]
async fn init_metrics_honours_the_none_exporter() {
    unsafe {
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://otel-collector:4318");
        std::env::set_var("OTEL_METRICS_EXPORTER", "none");
    }
    let provider = init_metrics(DEFAULT_SERVICE_NAME);
    unsafe {
        std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        std::env::remove_var("OTEL_METRICS_EXPORTER");
    }
    assert!(provider.is_none());
}

/// Shutting down a disabled pipeline is a no-op, not a panic.
#[tokio::test]
async fn shutdown_metrics_tolerates_a_disabled_pipeline() {
    shutdown_metrics(None).await;
}

/// `OTEL_SERVICE_NAME` beats the code default; the package version is always attached.
#[test]
#[serial]
fn resource_prefers_the_environment_service_name() {
    unsafe {
        std::env::set_var("OTEL_SERVICE_NAME", "extractor-discogs");
    }
    let resource = build_resource(DEFAULT_SERVICE_NAME);
    unsafe {
        std::env::remove_var("OTEL_SERVICE_NAME");
    }

    assert_eq!(resource.get(&opentelemetry::Key::from_static_str("service.name")).map(|v| v.to_string()), Some("extractor-discogs".to_string()));
    assert_eq!(
        resource.get(&opentelemetry::Key::from_static_str("service.version")).map(|v| v.to_string()),
        Some(env!("CARGO_PKG_VERSION").to_string())
    );
}

/// Without `OTEL_SERVICE_NAME` the package's canonical name is used.
#[test]
#[serial]
fn resource_falls_back_to_the_code_default_service_name() {
    unsafe {
        std::env::remove_var("OTEL_SERVICE_NAME");
    }
    let resource = build_resource(DEFAULT_SERVICE_NAME);
    assert_eq!(resource.get(&opentelemetry::Key::from_static_str("service.name")).map(|v| v.to_string()), Some(DEFAULT_SERVICE_NAME.to_string()));
}

/// `OTEL_RESOURCE_ATTRIBUTES` is merged in by the SDK's environment detector.
#[test]
#[serial]
fn resource_merges_otel_resource_attributes() {
    unsafe {
        std::env::set_var("OTEL_RESOURCE_ATTRIBUTES", "service.namespace=groovemap,deployment.environment.name=dev");
    }
    let resource = build_resource(DEFAULT_SERVICE_NAME);
    unsafe {
        std::env::remove_var("OTEL_RESOURCE_ATTRIBUTES");
    }

    assert_eq!(resource.get(&opentelemetry::Key::from_static_str("service.namespace")).map(|v| v.to_string()), Some("groovemap".to_string()));
    assert_eq!(resource.get(&opentelemetry::Key::from_static_str("deployment.environment.name")).map(|v| v.to_string()), Some("dev".to_string()));
}

/// The source label defaults to a placeholder rather than panicking when unset, and the
/// known provider interns to a static string.
#[test]
fn file_and_stage_labels_match_the_conventions() {
    assert_eq!(FileOutcome::Completed.as_str(), "completed");
    assert_eq!(FileOutcome::Skipped.as_str(), "skipped");
    assert_eq!(FileOutcome::Failed.as_str(), "failed");
    assert_eq!(Stage::Download.as_str(), "download");
    assert_eq!(Stage::Parse.as_str(), "parse");
    assert_eq!(Stage::Publish.as_str(), "publish");
    assert!(matches!(source(), SOURCE_DISCOGS | "unknown"));
}

// ---------------------------------------------------------------------------------------
// Runtime metrics
// ---------------------------------------------------------------------------------------

/// Export everything recorded so far and return `(name, attribute keys, value)` for every
/// scalar data point. Observable instruments are only collected during an export, so this
/// is also what drives their callbacks.
fn flush_numeric_observations() -> Vec<(String, BTreeSet<String>, f64)> {
    let harness = &*crate::telemetry::test_harness::HARNESS;
    harness.provider.force_flush().expect("force_flush should succeed for the in-memory exporter");

    fn keys<'a>(attributes: impl Iterator<Item = &'a opentelemetry::KeyValue>) -> BTreeSet<String> {
        attributes.map(|kv| kv.key.to_string()).collect()
    }

    let mut observations = Vec::new();
    for resource_metrics in harness.exporter.get_finished_metrics().expect("in-memory exporter should hand back its batches") {
        for scope in resource_metrics.scope_metrics() {
            for metric in scope.metrics() {
                let name = metric.name().to_string();
                match metric.data() {
                    AggregatedMetrics::U64(MetricData::Gauge(gauge)) => {
                        for point in gauge.data_points() {
                            observations.push((name.clone(), keys(point.attributes()), point.value() as f64));
                        }
                    }
                    AggregatedMetrics::F64(MetricData::Sum(sum)) => {
                        for point in sum.data_points() {
                            observations.push((name.clone(), keys(point.attributes()), point.value()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    observations
}

/// The most recent value reported for `name` under exactly `expected` attribute keys.
fn reported_value(observations: &[(String, BTreeSet<String>, f64)], name: &str, expected: &[&str]) -> f64 {
    let expected: BTreeSet<String> = expected.iter().map(|k| (*k).to_string()).collect();
    observations
        .iter()
        .filter(|(n, keys, _)| n == name && *keys == expected)
        .map(|(_, _, value)| *value)
        .next_back()
        .unwrap_or_else(|| {
            panic!(
                "instrument {name} was never exported with attribute keys {expected:?}; exported: {:?}",
                observations.iter().map(|(n, _, _)| n).collect::<BTreeSet<_>>()
            )
        })
}

/// The tokio gauges register on every target and report a plausible scheduler state; the
/// `/proc/self` instruments report plausible resource usage on Linux and are absent — not
/// zero — everywhere else.
#[tokio::test]
#[serial]
async fn runtime_instruments_report_plausible_values() {
    register_runtime_metrics();

    // `num_alive_tasks` counts SPAWNED tasks, and the test body itself is driven by
    // `block_on` rather than spawned — so park one real task on the runtime to give the
    // gauge something true to report, and release it once the export has been collected.
    let (release, parked) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let _ = parked.await;
    });
    tokio::task::yield_now().await;

    let observations = flush_numeric_observations();

    let _ = release.send(());
    task.await.expect("the parked task should finish cleanly");

    let workers = reported_value(&observations, METRIC_RUNTIME_TOKIO_WORKERS, &[]);
    assert!(workers >= 1.0, "a running tokio runtime has at least one worker, saw {workers}");

    let alive_tasks = reported_value(&observations, METRIC_RUNTIME_TOKIO_ALIVE_TASKS, &[]);
    assert!(alive_tasks >= 1.0, "the parked task must show up as alive, saw {alive_tasks}");

    let queue_depth = reported_value(&observations, METRIC_RUNTIME_TOKIO_GLOBAL_QUEUE_DEPTH, &[]);
    assert!(queue_depth >= 0.0, "the global queue depth cannot be negative, saw {queue_depth}");

    #[cfg(target_os = "linux")]
    {
        let user = reported_value(&observations, METRIC_PROCESS_CPU_TIME, &[ATTR_CPU_MODE]);
        assert!(user >= 0.0, "cpu time cannot be negative, saw {user}");

        let rss = reported_value(&observations, METRIC_PROCESS_MEMORY_USAGE, &[]);
        assert!(rss > 0.0, "a running process has a non-zero resident set, saw {rss}");

        let threads = reported_value(&observations, METRIC_PROCESS_THREAD_COUNT, &[]);
        assert!(threads >= 1.0, "a running process has at least one thread, saw {threads}");

        let fds = reported_value(&observations, METRIC_PROCESS_OPEN_FILE_DESCRIPTOR_COUNT, &[]);
        assert!(fds >= 3.0, "stdin/stdout/stderr are always open, saw {fds}");
    }

    #[cfg(not(target_os = "linux"))]
    for name in [
        METRIC_PROCESS_CPU_TIME,
        METRIC_PROCESS_MEMORY_USAGE,
        METRIC_PROCESS_THREAD_COUNT,
        METRIC_PROCESS_OPEN_FILE_DESCRIPTOR_COUNT,
    ] {
        assert!(
            !observations.iter().any(|(exported, _, _)| exported == name),
            "{name} must not be registered off Linux — /proc/self does not exist there"
        );
    }
}

/// `process.cpu.time` carries both accounting modes, and only `cpu.mode`.
#[cfg(target_os = "linux")]
#[tokio::test]
#[serial]
async fn process_cpu_time_is_split_by_mode() {
    register_runtime_metrics();
    let observations = flush_numeric_observations();

    let modes: BTreeSet<String> = observations
        .iter()
        .filter(|(name, _, _)| name == METRIC_PROCESS_CPU_TIME)
        .flat_map(|(_, keys, _)| keys.iter().cloned())
        .collect();
    assert_eq!(modes, BTreeSet::from([ATTR_CPU_MODE.to_string()]), "cpu.mode is the only attribute on process.cpu.time");
}

/// A disabled bootstrap installs no observable callbacks at all, and does not panic.
#[tokio::test]
#[serial]
async fn init_metrics_disabled_registers_no_runtime_instruments() {
    unsafe {
        std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        std::env::remove_var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT");
        std::env::set_var("OTEL_METRICS_EXPORTER", "none");
    }
    let registered_before = crate::telemetry::runtime_metrics_registered();
    let provider = init_metrics(DEFAULT_SERVICE_NAME);
    let registered_after = crate::telemetry::runtime_metrics_registered();
    unsafe {
        std::env::remove_var("OTEL_METRICS_EXPORTER");
    }

    assert!(provider.is_none());
    assert_eq!(registered_after, registered_before, "a disabled bootstrap must not register runtime instruments");
}

/// The `/proc/self/stat` field walk survives a `comm` containing spaces and parentheses.
#[cfg(target_os = "linux")]
#[test]
fn process_cpu_seconds_parses_an_awkward_comm() {
    let fields: Vec<String> = (3..=52).map(|n| n.to_string()).collect();
    let stat = format!("1 (weird ) name) S {}", fields[1..].join(" "));
    // Fields 14 and 15 in that synthetic line are the literal numbers 14 and 15.
    let (user, system) = crate::telemetry::parse_process_cpu_seconds(&stat).expect("stat line should parse");
    assert_eq!(user, 14.0 / 100.0);
    assert_eq!(system, 15.0 / 100.0);
}

/// A truncated `/proc/self/stat` yields no observation rather than a panic or a zero.
#[cfg(target_os = "linux")]
#[test]
fn process_cpu_seconds_rejects_a_truncated_stat_line() {
    assert!(crate::telemetry::parse_process_cpu_seconds("1 (extractor) S 3 4 5").is_none());
    assert!(crate::telemetry::parse_process_cpu_seconds("").is_none());
}

/// `VmRSS` and `Threads` are read out of a `/proc/self/status` body by name.
#[cfg(target_os = "linux")]
#[test]
fn status_fields_are_read_by_name() {
    let status = "Name:\textractor\nThreads:\t9\nVmRSS:\t   12345 kB\n";
    assert_eq!(crate::telemetry::read_status_field(status, "Threads:"), Some(9));
    assert_eq!(crate::telemetry::read_status_field(status, "VmRSS:"), Some(12345));
    assert_eq!(crate::telemetry::read_status_field(status, "VmHWM:"), None);
}
