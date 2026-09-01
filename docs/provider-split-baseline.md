# Provider-split compatibility baseline

This document freezes the observable `catalog-ingestion` behavior at the boundary before
Discogs and MusicBrainz are separated into provider repositories. The current combined binary
must retain the same contract and lifecycle characterizations without changing events, state,
health, coordination, or shutdown behavior. Repository cutover has one explicit scheduling
exception: provider-owned containers remove the MusicBrainz-side Discogs-health compatibility
wait and may ingest concurrently. Event, state, health, and shutdown compatibility remains
unchanged.

## Recorded boundary

- Starting revision: `4ff2accc8a8e20ea1c5c4111ca45965ece9f7b92`
- Catalog contract: `contracts/catalog-events/v1/contract.json`
- Contract SHA-256: `cfb094491b2a29ab5b3ba0078476387cd29c881535464cdecc82dcbb6d5fed03`
- Contract version: `groovemap.catalog-events` v1

The digest is asserted by `tests/provider_split_baseline_test.rs`. Changing it is a contract
decision, not routine regeneration: update the versioning and consumer rollout plan before
moving this baseline.

## Wire and fixture boundary

The split-baseline test freezes the tagged v1 `data`, `file_complete`, and
`extraction_complete` envelopes; the Discogs and MusicBrainz entity vocabularies; the fanout
exchange naming template; and the default `groovemap-discogs` and `groovemap-musicbrainz`
exchange prefixes. It also deserializes every generated representative data fixture plus the
file- and extraction-complete fixtures through the producer's `Message` type.

Discogs records are normalized before publication and receive a deterministic SHA-256 of the
normalized payload. The baseline test pins both a representative normalized artist payload and
its hash. MusicBrainz parsers currently emit an empty `sha256`; the same test explicitly pins
that behavior for artists, labels, release groups, and releases. The split must preserve these
different behaviors until a separately coordinated contract change replaces them.

## Lifecycle and state boundary

The maintained Rust suite provides the executable evidence for the remaining compatibility
surface:

| Behavior to preserve | Characterization evidence |
| --- | --- |
| Publish `file_complete` before marking a file complete | `tests/extractor_di_test.rs::test_process_single_file_amqp_failure_at_send_file_complete_leaves_marker_not_completed` |
| Publish `extraction_complete` before marking a version complete, including resume | `tests/extractor_di_test.rs::test_process_discogs_data_resumed_completion_amqp_failure_stays_retryable` and `test_process_discogs_data_extraction_complete_failure` |
| Keep the original processing start time on resume | `src/tests/state_marker_tests.rs::test_resume_preserves_original_processing_start_time` and `test_all_files_complete_before_crash_keeps_original_start_time` |
| Retain provider-specific marker paths | `tests/provider_split_baseline_test.rs::provider_marker_filenames_are_frozen` |
| Load legacy marker JSON without checksum fields | `src/tests/state_marker_tests.rs::test_legacy_marker_json_without_checksums_loads` |
| Preserve health readiness/status and trigger semantics | `src/tests/health_tests.rs::test_ready_handler_transitions`, `test_health_includes_extraction_status`, and the `test_trigger_handler_*` cases |
| Preserve trigger polling and one-shot consumption | `src/tests/extractor_tests.rs::test_wait_for_trigger_returns_when_triggered`, `test_wait_for_trigger_clears_flag`, and `test_wait_for_trigger_only_fires_once` |
| Fail open after ten unreachable Discogs-health attempts | `src/tests/extractor_tests.rs::wait_for_discogs_idle_tests::test_proceeds_after_max_unreachable_retries` |
| Escalate and cap unreachable-health backoff | `src/tests/extractor_tests.rs::discogs_unreachable_backoff_tests::test_escalates_and_caps` |
| Poll while Discogs reports `running`; proceed for terminal states | `src/tests/extractor_tests.rs::wait_for_discogs_idle_tests::test_waits_then_proceeds_when_running_then_idle` and its idle/completed/failed/waiting cases |
| Make busy polling and unreachable backoff shutdown-aware | `src/tests/extractor_tests.rs::wait_for_discogs_idle_tests::test_shutdown_while_discogs_busy` and `test_shutdown_during_unreachable_retry_backoff` |
| Avoid starting or finalizing provider work under shutdown | `tests/extractor_di_test.rs::test_process_discogs_data_shutdown_before_files_does_not_finalize` and `test_musicbrainz_run_bails_under_shutdown` |

## Validation observation

At the planning boundary, the Rust suites passed. In a clean shell without Mise activation,
`just check` then failed when the Justfile reached a bare `python` invocation because `python`
was not resolvable. That shell/toolchain failure is recorded as evidence about the starting
environment; it is not a Rust behavior change and must not be mistaken for a provider-split
regression.
