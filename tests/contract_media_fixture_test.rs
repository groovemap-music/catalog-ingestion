//! Guards the Discogs `releases` contract fixture's `media` block against the mapper.
//!
//! `contracts/catalog-events/definitions/discogs.json` carries a hand-authored `formats`
//! payload and the `media` block it is expected to produce; `just contract` copies both,
//! verbatim, into `contracts/catalog-events/v1/fixtures/discogs-releases.data.json`. This
//! test runs the fixture's `formats` back through `src/discogs/media.rs` -- the same mapper
//! `message_normalizer` attaches to every published release -- and demands the fixture's
//! `media` match exactly, so the fixture and the mapper can never silently drift apart.

use extractor::discogs::media::map_discogs_formats;
use serde_json::Value;

const RELEASES_FIXTURE: &str = include_str!("../contracts/catalog-events/v1/fixtures/discogs-releases.data.json");

#[test]
fn test_releases_fixture_media_matches_the_mapper() {
    let fixture: Value = serde_json::from_str(RELEASES_FIXTURE).expect("the releases fixture is valid JSON");
    let formats = fixture.get("formats").expect("the releases fixture carries a formats payload");
    let expected = fixture.get("media").expect("the releases fixture carries the expected media block");

    let produced = serde_json::to_value(map_discogs_formats(Some(formats))).expect("the media block serializes");

    assert_eq!(
        &produced,
        expected,
        "the fixture's media block no longer matches src/discogs/media.rs -- regenerate it with `just contract` \
         after updating definitions/discogs.json\n  produced: {}\n  expected: {}",
        serde_json::to_string_pretty(&produced).unwrap_or_default(),
        serde_json::to_string_pretty(expected).unwrap_or_default()
    );
}
