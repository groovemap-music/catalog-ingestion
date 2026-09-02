// Library exports for testing

pub mod config;
pub mod discogs;
pub mod generated {
    pub mod catalog_contract;
}
pub mod extractor;
pub mod health;
pub mod message_queue;
pub mod musicbrainz;
pub mod polite_http;
pub mod runtime;
pub mod state_marker;
pub mod types;

// Frozen compatibility paths for downstream tests and the current combined binary.
// Provider-owned code lives under `discogs` / `musicbrainz`; new callers should use
// those boundaries directly.
pub use discogs::downloader as discogs_downloader;
pub use discogs::normalize;
pub use discogs::parser;
pub use discogs::rules;
pub use musicbrainz::downloader as musicbrainz_downloader;
pub use musicbrainz::jsonl_parser;

// Additional test modules
#[cfg(test)]
#[path = "tests/message_queue_unit_tests.rs"]
mod message_queue_unit_tests;
