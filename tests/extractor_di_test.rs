//! Source-owned extractor integration-test entry points.

mod mock_helpers;

#[path = "extractor_di/discogs.rs"]
mod discogs;

#[path = "extractor_di/musicbrainz.rs"]
mod musicbrainz;
