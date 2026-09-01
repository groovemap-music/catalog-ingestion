//! Compatibility exports for callers of the pre-partition `extractor` module.
//!
//! New composition code should depend on `discogs`, `musicbrainz`, or
//! `runtime` directly. No provider policy or orchestration belongs here.

pub use crate::discogs::{message_normalizer, message_validator, process_discogs_data, process_single_file, run_extraction_loop};
pub use crate::musicbrainz::combined_runtime_compat::{WaitOutcome, wait_for_discogs_idle, wait_for_discogs_idle_with_interval};
pub use crate::musicbrainz::{process_musicbrainz_data, run_musicbrainz_loop};
pub use crate::runtime::{
    BatcherConfig, DefaultMessageQueueFactory, ExtractionStatus, ExtractorState, MessageQueueFactory, message_batcher, message_publisher,
};
