//! Executable compatibility boundary for the provider repository split.
//!
//! These tests deliberately repeat a small amount of lower-level coverage so a future split can
//! verify the producer-owned wire contract without reconstructing behavior from implementation
//! details. Operational lifecycle behavior remains covered by the focused tests cataloged in
//! `docs/provider-split-baseline.md`.

use extractor::generated::catalog_contract::{
    AMQP_EXCHANGE_TYPE, CONTRACT_NAME, CONTRACT_VERSION, DEFAULT_DISCOGS_EXCHANGE_PREFIX, DEFAULT_MUSICBRAINZ_EXCHANGE_PREFIX, DISCOGS_ENTITY_TYPES,
    MUSICBRAINZ_ENTITY_TYPES,
};
use extractor::jsonl_parser::{parse_mb_artist_line, parse_mb_label_line, parse_mb_release_group_line, parse_mb_release_line};
use extractor::normalize::normalize_record;
use extractor::state_marker::StateMarker;
use extractor::types::{Message, calculate_content_hash};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const CONTRACT: &str = include_str!("../contracts/catalog-events/v1/contract.json");
const CONTRACT_SHA256: &str = "cfb094491b2a29ab5b3ba0078476387cd29c881535464cdecc82dcbb6d5fed03";

#[test]
fn v1_contract_digest_envelope_vocabularies_and_defaults_are_frozen() {
    assert_eq!(hex::encode(Sha256::digest(CONTRACT.as_bytes())), CONTRACT_SHA256);

    let contract: Value = serde_json::from_str(CONTRACT).unwrap();
    assert_eq!(contract["contract"], "groovemap.catalog-events");
    assert_eq!(contract["version"], 1);
    assert_eq!(contract["exchange"]["kind"], "fanout");
    assert_eq!(contract["exchange"]["name_template"], "{exchange_prefix}-{entity}");

    assert_eq!(CONTRACT_NAME, "groovemap.catalog-events");
    assert_eq!(CONTRACT_VERSION, 1);
    assert_eq!(AMQP_EXCHANGE_TYPE, "fanout");
    assert_eq!(DISCOGS_ENTITY_TYPES, ["artists", "labels", "masters", "releases"]);
    assert_eq!(MUSICBRAINZ_ENTITY_TYPES, ["artists", "labels", "release-groups", "releases"]);
    assert_eq!(DEFAULT_DISCOGS_EXCHANGE_PREFIX, "groovemap-discogs");
    assert_eq!(DEFAULT_MUSICBRAINZ_EXCHANGE_PREFIX, "groovemap-musicbrainz");

    let schema: Value = serde_json::from_str(include_str!("../contracts/catalog-events/v1/schemas/event.schema.json")).unwrap();
    assert_eq!(schema["$defs"]["data"]["required"], json!(["type", "id", "sha256"]));
    assert_eq!(schema["$defs"]["file_complete"]["required"], json!(["type", "data_type", "timestamp", "total_processed", "file"]));
    assert_eq!(schema["$defs"]["extraction_complete"]["required"], json!(["type", "version", "timestamp", "started_at", "record_counts"]));
}

#[test]
fn representative_v1_fixtures_deserialize_through_the_producer_envelope() {
    let data_fixtures = [
        include_str!("../contracts/catalog-events/v1/fixtures/discogs-artists.data.json"),
        include_str!("../contracts/catalog-events/v1/fixtures/discogs-labels.data.json"),
        include_str!("../contracts/catalog-events/v1/fixtures/discogs-masters.data.json"),
        include_str!("../contracts/catalog-events/v1/fixtures/discogs-releases.data.json"),
        include_str!("../contracts/catalog-events/v1/fixtures/musicbrainz-artists.data.json"),
        include_str!("../contracts/catalog-events/v1/fixtures/musicbrainz-labels.data.json"),
        include_str!("../contracts/catalog-events/v1/fixtures/musicbrainz-release-groups.data.json"),
        include_str!("../contracts/catalog-events/v1/fixtures/musicbrainz-releases.data.json"),
    ];

    for fixture in data_fixtures {
        let Message::Data(message) = serde_json::from_str(fixture).unwrap() else {
            panic!("expected data fixture");
        };
        assert!(message.id.starts_with("contract-"));
    }

    let Message::FileComplete(file_complete) =
        serde_json::from_str(include_str!("../contracts/catalog-events/v1/fixtures/file-complete.json")).unwrap()
    else {
        panic!("expected file_complete fixture");
    };
    assert_eq!(file_complete.data_type, "artists");
    assert_eq!(file_complete.file, "contract-artists.xml.gz");
    assert_eq!(file_complete.total_processed, 1);

    let Message::ExtractionComplete(extraction_complete) =
        serde_json::from_str(include_str!("../contracts/catalog-events/v1/fixtures/extraction-complete.json")).unwrap()
    else {
        panic!("expected extraction_complete fixture");
    };
    assert_eq!(extraction_complete.version, "contract-fixture");
    assert_eq!(extraction_complete.record_counts["artists"], 1);
}

#[test]
fn discogs_normalized_payload_and_hash_are_frozen() {
    let mut payload = json!({
        "id": "1",
        "name": "Aphex Twin",
        "members": {"name": [{"@id": "7", "#text": "Richard D. James"}]}
    });

    normalize_record("artists", &mut payload);

    assert_eq!(
        payload,
        json!({
            "id": "1",
            "members": [{"id": "7", "name": "Richard D. James"}],
            "name": "Aphex Twin"
        })
    );
    assert_eq!(calculate_content_hash(&payload), "f0e7be1d0e7c3f56326eae1015c01eb81e896a4a70d27cc6931947571f9a8dfd");
}

#[test]
fn musicbrainz_parser_hashes_remain_empty() {
    let messages = [
        parse_mb_artist_line(r#"{"id":"mb-artist","name":"Artist","sort-name":"Artist","relations":[]}"#).unwrap(),
        parse_mb_label_line(r#"{"id":"mb-label","name":"Label","relations":[]}"#).unwrap(),
        parse_mb_release_group_line(r#"{"id":"mb-release-group","title":"Release Group","relations":[]}"#).unwrap(),
        parse_mb_release_line(r#"{"id":"mb-release","title":"Release","relations":[]}"#).unwrap(),
    ];

    for message in messages {
        assert!(message.sha256.is_empty());
    }
}

#[test]
fn provider_marker_filenames_are_frozen() {
    assert_eq!(StateMarker::file_path(Path::new("/discogs-data"), "20260101"), PathBuf::from("/discogs-data/.extraction_status_20260101.json"));
    assert_eq!(
        StateMarker::musicbrainz_file_path(Path::new("/musicbrainz-data"), "20260326-001001"),
        PathBuf::from("/musicbrainz-data/20260326-001001/.mb_extraction_status_20260326-001001.json")
    );
}
