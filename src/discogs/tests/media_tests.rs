//! Canonical media block (ADR 0007) mapper tests.
//!
//! The conformance suite in `fixtures/media/` is vendored verbatim from the design
//! repository's `taxonomy/media/v1/fixtures/`. Those input/expected pairs are the contract
//! between this producer, the MusicBrainz producer, and the shared Python mapper: all three
//! must reproduce the design repository's reference mapper exactly. Never edit a fixture to
//! make this code pass — re-vendor the suite when the vocabulary changes.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::discogs::media::{attach_media_block, map_discogs_formats};
use crate::discogs::message_normalizer;
use crate::types::{DataMessage, DataType};

/// The vendored conformance suite: 19 pairs at the pinned design commit, of which the 12
/// `discogs-*` ones exercise this producer. The seven `musicbrainz-*` pairs are carried
/// unchanged so the vendored suite matches the pinned upstream set file for file; the
/// MusicBrainz producer owns their mapper.
const FIXTURE_TOTAL: usize = 19;
const DISCOGS_FIXTURE_TOTAL: usize = 12;

fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/discogs/tests/fixtures/media")
}

fn fixtures() -> Vec<(String, Value)> {
    let mut loaded: Vec<(String, Value)> = fs::read_dir(fixture_directory())
        .expect("the vendored media fixture directory is readable")
        .map(|entry| entry.expect("the fixture directory entry is readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .map(|path| {
            let name = path.file_name().expect("the fixture has a file name").to_string_lossy().to_string();
            let text = fs::read_to_string(&path).expect("the fixture is readable");
            let value: Value = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{name} is valid JSON: {error}"));
            (name, value)
        })
        .collect();
    loaded.sort_by(|left, right| left.0.cmp(&right.0));
    loaded
}

/// Map a raw format list the way the fixtures express it.
fn map(formats: &Value) -> Value {
    serde_json::to_value(map_discogs_formats(Some(formats))).expect("the media block serializes")
}

// ── Conformance ─────────────────────────────────────────────────────

/// Guard the vendored suite itself: a fixture silently lost or added would otherwise make
/// the conformance test below vacuously weaker.
#[test]
fn test_vendored_fixture_suite_is_complete() {
    let all = fixtures();
    let discogs = all.iter().filter(|(_, fixture)| fixture["provider"] == json!("discogs")).count();
    assert_eq!(all.len(), FIXTURE_TOTAL, "the vendored suite must match the pinned design commit file for file");
    assert_eq!(discogs, DISCOGS_FIXTURE_TOTAL, "every discogs fixture must be present");
}

/// Every Discogs conformance pair: run the fixture's raw input through the mapper and demand
/// the exact block the design repository's reference mapper produces, field for field.
#[test]
fn test_discogs_conformance_fixtures() {
    let mut checked = 0;
    for (name, fixture) in fixtures() {
        if fixture["provider"] != json!("discogs") {
            continue;
        }
        let formats = fixture["input"].get("formats");
        let produced = serde_json::to_value(map_discogs_formats(formats)).expect("the media block serializes");
        let expected = &fixture["expected"];
        assert_eq!(
            &produced,
            expected,
            "fixture {name} does not match the reference mapper\n  produced: {}\n  expected: {}",
            serde_json::to_string_pretty(&produced).unwrap_or_default(),
            serde_json::to_string_pretty(expected).unwrap_or_default()
        );
        checked += 1;
    }
    assert_eq!(checked, DISCOGS_FIXTURE_TOTAL, "every discogs fixture must be exercised");
}

// ── Shape guarantees ────────────────────────────────────────────────

/// A release the dump gave no formats still carries the block, empty — never a missing key,
/// so no consumer has to branch on its absence.
#[test]
fn test_absent_formats_produce_the_empty_block() {
    let block = map_discogs_formats(None);
    let produced = serde_json::to_value(&block).expect("the media block serializes");
    assert_eq!(
        produced,
        json!({
            "taxonomy_version": "1",
            "items": [],
            "families": [],
            "release_kind": null,
            "traits": [],
            "edition": [],
            "packaging": null,
            "container": null,
            "flags": [],
            "unmapped": {"formats": [], "descriptions": []}
        })
    );
}

/// A `formats` value that is not a list is treated as no formats rather than panicking.
#[test]
fn test_non_array_formats_produce_the_empty_block() {
    for formats in [json!(null), json!("Vinyl"), json!({"format": []}), json!(7)] {
        let block = map_discogs_formats(Some(&formats));
        assert!(block.items.is_empty(), "{formats} must yield no items");
        assert!(block.families.is_empty(), "{formats} must yield no families");
    }
}

/// Every field is present with an explicit null or empty list, so two implementations
/// serialize the same bytes.
#[test]
fn test_every_field_is_always_present() {
    let produced = map(&json!([{"name": "CD", "qty": "1"}]));
    let block = produced.as_object().expect("the block is an object");
    for key in [
        "taxonomy_version",
        "items",
        "families",
        "release_kind",
        "traits",
        "edition",
        "packaging",
        "container",
        "flags",
        "unmapped",
    ] {
        assert!(block.contains_key(key), "the block must carry {key}");
    }
    let item = produced["items"][0].as_object().expect("the item is an object");
    for key in [
        "family",
        "medium",
        "qty",
        "size_inches",
        "speed_rpm",
        "channels",
        "codec",
        "variants",
        "appearance",
        "position",
        "track_count",
        "source",
    ] {
        assert!(item.contains_key(key), "the item must carry {key}");
    }
    let source = produced["items"][0]["source"].as_object().expect("the source is an object");
    for key in ["provider", "name", "descriptions", "text"] {
        assert!(source.contains_key(key), "the source must carry {key}");
    }
}

// ── Single, multiple, and repeated formats ──────────────────────────

#[test]
fn test_single_format_maps_to_one_item() {
    let produced = map(&json!([{"name": "CD", "qty": "1", "descriptions": {"description": ["Album"]}}]));
    assert_eq!(produced["items"].as_array().expect("items is a list").len(), 1);
    assert_eq!(produced["items"][0]["family"], json!("optical"));
    assert_eq!(produced["items"][0]["medium"], json!("optical_cd"));
    assert_eq!(produced["items"][0]["qty"], json!(1));
    assert_eq!(produced["families"], json!(["optical"]));
    assert_eq!(produced["release_kind"], json!("album"));
    assert_eq!(produced["items"][0]["source"]["provider"], json!("discogs"));
}

/// Items keep source order even though `families` is sorted, and each item's descriptions
/// route to that item alone.
#[test]
fn test_multiple_formats_keep_source_order_with_sorted_families() {
    let produced = map(&json!([
        {"name": "Vinyl", "qty": "1", "descriptions": {"description": ["12\"", "45 RPM"]}},
        {"name": "CD", "qty": "1", "descriptions": {"description": ["Mono"]}},
        {"name": "Cassette", "qty": "1"}
    ]));
    let media: Vec<&Value> = produced["items"].as_array().expect("items is a list").iter().map(|item| &item["medium"]).collect();
    assert_eq!(media, vec![&json!("vinyl_12"), &json!("optical_cd"), &json!("tape_cassette")]);
    assert_eq!(produced["families"], json!(["optical", "tape", "vinyl"]), "families are sorted and de-duplicated");
    assert_eq!(produced["items"][0]["speed_rpm"], json!(45), "a descriptor applies to its own item");
    assert_eq!(produced["items"][1]["speed_rpm"], json!(null));
    assert_eq!(produced["items"][1]["channels"], json!("mono"));
    assert_eq!(produced["items"][0]["channels"], json!(null));
}

/// The same family twice is one family entry but two items.
#[test]
fn test_repeated_family_deduplicates_families_only() {
    let produced = map(&json!([
        {"name": "Vinyl", "qty": "1", "descriptions": {"description": ["7\""]}},
        {"name": "Vinyl", "qty": "1", "descriptions": {"description": ["12\""]}}
    ]));
    assert_eq!(produced["items"].as_array().expect("items is a list").len(), 2);
    assert_eq!(produced["families"], json!(["vinyl"]));
}

#[test]
fn test_qty_greater_than_one_is_a_json_integer() {
    let produced = map(&json!([{"name": "Vinyl", "qty": "3", "descriptions": {"description": ["LP"]}}]));
    assert_eq!(produced["items"][0]["qty"], json!(3));
    assert!(produced["items"][0]["qty"].is_u64(), "qty is a JSON number, not the source string");
}

/// A missing, unparseable, or non-positive quantity counts as a single unit.
#[test]
fn test_qty_falls_back_to_one() {
    for qty in [json!(null), json!(""), json!("0"), json!("-2"), json!("many")] {
        let produced = map(&json!([{"name": "CD", "qty": qty}]));
        assert_eq!(produced["items"][0]["qty"], json!(1), "qty {qty} must count as one unit");
    }
    let missing = map(&json!([{"name": "CD"}]));
    assert_eq!(missing["items"][0]["qty"], json!(1), "an absent qty must count as one unit");
}

/// The dump carries `qty` as a string; the API sometimes as a number. Both read the same.
#[test]
fn test_qty_accepts_the_numeric_api_shape() {
    let produced = map(&json!([{"name": "CD", "qty": 4}]));
    assert_eq!(produced["items"][0]["qty"], json!(4));
}

// ── Description handling ────────────────────────────────────────────

/// No descriptions at all: the item still resolves, through the family's unspecified medium.
#[test]
fn test_missing_descriptions_still_produce_an_item() {
    let produced = map(&json!([{"name": "Vinyl", "qty": "1"}]));
    assert_eq!(produced["items"][0]["medium"], json!("vinyl_unspecified"));
    assert_eq!(produced["items"][0]["source"]["descriptions"], json!([]));
    assert_eq!(produced["items"][0]["source"]["text"], json!(null));
    assert_eq!(produced["release_kind"], json!(null));
}

/// An empty description container is the same as none.
#[test]
fn test_empty_description_container_is_the_same_as_none() {
    for descriptions in [json!({}), json!({"description": []}), json!([]), json!(null)] {
        let produced = map(&json!([{"name": "CD", "qty": "1", "descriptions": descriptions}]));
        assert_eq!(produced["items"][0]["source"]["descriptions"], json!([]), "{descriptions} must flatten to nothing");
    }
}

/// A single description arrives as a bare string in the normalized dump shape.
#[test]
fn test_single_description_string_is_flattened() {
    let produced = map(&json!([{"name": "CD", "qty": "1", "descriptions": {"description": "Album"}}]));
    assert_eq!(produced["items"][0]["source"]["descriptions"], json!(["Album"]));
    assert_eq!(produced["release_kind"], json!("album"));
}

/// The Discogs API returns descriptions as a flat array; both shapes map identically.
#[test]
fn test_flat_and_wrapped_description_shapes_agree() {
    let wrapped = map(&json!([{"name": "Vinyl", "qty": "1", "descriptions": {"description": ["12\"", "Album"]}}]));
    let flat = map(&json!([{"name": "Vinyl", "qty": "1", "descriptions": ["12\"", "Album"]}]));
    assert_eq!(wrapped, flat);
}

/// Descriptions keep their source order and duplicates in the provenance record, while the
/// facts they produce are sorted and de-duplicated.
#[test]
fn test_source_descriptions_are_verbatim_while_facts_are_ordered() {
    let produced = map(&json!([{"name": "CD", "qty": "1", "descriptions": {"description": ["Remastered", "Reissue", "Remastered"]}}]));
    assert_eq!(produced["items"][0]["source"]["descriptions"], json!(["Remastered", "Reissue", "Remastered"]));
    assert_eq!(produced["edition"], json!(["reissue", "remastered"]));
}

/// Non-string description entries are skipped rather than stringified into `unmapped`.
#[test]
fn test_non_string_descriptions_are_skipped() {
    let produced = map(&json!([{"name": "CD", "qty": "1", "descriptions": {"description": ["Album", 7, null, {"x": 1}]}}]));
    assert_eq!(produced["items"][0]["source"]["descriptions"], json!(["Album"]));
    assert_eq!(produced["unmapped"]["descriptions"], json!([]));
}

/// The first value wins for a scalar release fact, so a conflicting later descriptor cannot
/// rewrite the release kind.
#[test]
fn test_first_value_wins_for_scalar_release_facts() {
    let produced = map(&json!([{"name": "CD", "qty": "1", "descriptions": {"description": ["Album", "Single", "EP"]}}]));
    assert_eq!(produced["release_kind"], json!("album"));
}

/// A descriptor the source states beats the medium's default.
#[test]
fn test_stated_attribute_beats_the_medium_default() {
    let produced = map(&json!([{"name": "Shellac", "qty": "1", "descriptions": {"description": ["10\"", "45 RPM"]}}]));
    assert_eq!(produced["items"][0]["medium"], json!("shellac_10"));
    assert_eq!(produced["items"][0]["speed_rpm"], json!(45), "the stated speed wins over the 78 RPM default");
    assert_eq!(produced["items"][0]["size_inches"], json!(10));
}

/// A fractional speed stays a JSON number, not a string.
#[test]
fn test_fractional_speed_is_a_json_number() {
    let produced = map(&json!([{"name": "Vinyl", "qty": "1", "descriptions": {"description": ["12\"", "33 ⅓ RPM"]}}]));
    assert_eq!(produced["items"][0]["speed_rpm"], json!(33.33));
    assert!(produced["items"][0]["speed_rpm"].is_f64());
}

// ── Entries that produce no item ────────────────────────────────────

/// A format name the vocabulary does not know produces no item and is preserved verbatim,
/// while the release facts its descriptors carry still apply.
#[test]
fn test_unknown_format_name_is_preserved() {
    let produced = map(&json!([{"name": "Holographic Cube", "qty": "1", "descriptions": {"description": ["Album", "Unheard Of Descriptor"]}}]));
    assert_eq!(produced["items"], json!([]));
    assert_eq!(produced["families"], json!([]));
    assert_eq!(produced["unmapped"]["formats"], json!(["Holographic Cube"]));
    assert_eq!(produced["unmapped"]["descriptions"], json!(["Unheard Of Descriptor"]));
    assert_eq!(produced["release_kind"], json!("album"), "release facts survive an unknown medium");
}

/// Unknown values are sorted and de-duplicated so the same release always hashes the same.
#[test]
fn test_unmapped_values_are_sorted_and_deduplicated() {
    let produced = map(&json!([
        {"name": "Zeta Sphere", "qty": "1", "descriptions": {"description": ["Nonsense", "Gibberish"]}},
        {"name": "Alpha Sphere", "qty": "1", "descriptions": {"description": ["Nonsense"]}},
        {"name": "Zeta Sphere", "qty": "1"}
    ]));
    assert_eq!(produced["unmapped"]["formats"], json!(["Alpha Sphere", "Zeta Sphere"]));
    assert_eq!(produced["unmapped"]["descriptions"], json!(["Gibberish", "Nonsense"]));
}

/// A format entry without a name is neither an item nor an unmapped value: there is nothing
/// to record.
#[test]
fn test_format_without_a_name_records_nothing() {
    let produced = map(&json!([{"qty": "1", "descriptions": {"description": ["Album"]}}]));
    assert_eq!(produced["items"], json!([]));
    assert_eq!(produced["unmapped"]["formats"], json!([]));
    assert_eq!(produced["release_kind"], json!("album"));
}

/// Non-object entries in the formats list are skipped without disturbing their siblings.
#[test]
fn test_non_object_format_entries_are_skipped() {
    let produced = map(&json!([null, "Vinyl", 42, ["CD"], {"name": "CD", "qty": "2", "descriptions": {"description": ["Album"]}}, true]));
    assert_eq!(produced["items"].as_array().expect("items is a list").len(), 1);
    assert_eq!(produced["items"][0]["medium"], json!("optical_cd"));
    assert_eq!(produced["items"][0]["qty"], json!(2));
    assert_eq!(produced["unmapped"]["formats"], json!([]), "a non-object entry is not an unmapped format name");
}

/// `Box Set` is a container, never a medium: it contributes no item, and the set's real
/// media arrive as sibling entries.
#[test]
fn test_box_set_is_a_container_not_a_medium() {
    let produced = map(&json!([
        {"name": "Box Set", "qty": "1", "descriptions": {"description": ["Limited Edition"]}},
        {"name": "CD", "qty": "3", "descriptions": {"description": ["Album"]}}
    ]));
    assert_eq!(produced["container"], json!("box_set"));
    assert_eq!(produced["items"].as_array().expect("items is a list").len(), 1);
    assert_eq!(produced["items"][0]["medium"], json!("optical_cd"));
    assert_eq!(produced["edition"], json!(["limited"]));
}

/// `All Media` is a release-level flag, never a medium.
#[test]
fn test_all_media_is_a_flag_not_a_medium() {
    let produced = map(&json!([{"name": "All Media", "qty": "1", "descriptions": {"description": ["Promo"]}}]));
    assert_eq!(produced["flags"], json!(["all_media"]));
    assert_eq!(produced["items"], json!([]));
    assert_eq!(produced["edition"], json!(["promo"]));
}

/// An item-targeted descriptor on a container entry has no item to land on and is dropped
/// rather than leaking onto a sibling medium.
#[test]
fn test_item_descriptors_on_a_container_entry_do_not_leak() {
    let produced = map(&json!([
        {"name": "Box Set", "qty": "1", "descriptions": {"description": ["Picture Disc", "Stereo"]}},
        {"name": "Vinyl", "qty": "1", "descriptions": {"description": ["LP"]}}
    ]));
    assert_eq!(produced["items"][0]["appearance"], json!([]));
    assert_eq!(produced["items"][0]["channels"], json!(null));
}

// ── Medium resolution ───────────────────────────────────────────────

/// A family named without a size resolves to its unspecified medium.
#[test]
fn test_family_without_a_size_resolves_to_unspecified() {
    let produced = map(&json!([{"name": "Vinyl", "qty": "1", "descriptions": {"description": ["Album"]}}]));
    assert_eq!(produced["items"][0]["family"], json!("vinyl"));
    assert_eq!(produced["items"][0]["medium"], json!("vinyl_unspecified"));
    assert_eq!(produced["items"][0]["size_inches"], json!(null));
}

/// A size the family cannot resolve also falls back to the unspecified medium, keeping the
/// stated size on the item.
#[test]
fn test_unresolvable_size_keeps_the_size_and_falls_back() {
    let produced = map(&json!([{"name": "Vinyl", "qty": "1", "descriptions": {"description": ["16\""]}}]));
    assert_eq!(produced["items"][0]["medium"], json!("vinyl_unspecified"));
    assert_eq!(produced["items"][0]["size_inches"], json!(16));
}

/// A format that names a medium outright infers its family from the vocabulary.
#[test]
fn test_medium_entry_infers_its_family() {
    let produced = map(&json!([{"name": "File", "qty": "1", "descriptions": {"description": ["FLAC"]}}]));
    assert_eq!(produced["items"][0]["family"], json!("digital"));
    assert_eq!(produced["items"][0]["medium"], json!("digital_file"));
    assert_eq!(produced["items"][0]["codec"], json!("flac"));
}

/// `Hybrid` is the SACD medium plus the hybrid-layer variant, and variants are sorted and
/// de-duplicated.
#[test]
fn test_hybrid_carries_the_variant() {
    let produced = map(&json!([{"name": "Hybrid", "qty": "1", "descriptions": {"description": ["SACD", "Multichannel"]}}]));
    assert_eq!(produced["items"][0]["medium"], json!("optical_sacd"));
    assert_eq!(produced["items"][0]["variants"], json!(["hybrid_layer"]));
    assert_eq!(produced["items"][0]["channels"], json!("multichannel"));
}

/// The free-text field is kept verbatim on the item's provenance and never parsed.
#[test]
fn test_free_text_is_kept_verbatim() {
    let produced = map(&json!([{"name": "Vinyl", "qty": "1", "text": "Blue Vinyl", "descriptions": {"description": ["LP"]}}]));
    assert_eq!(produced["items"][0]["source"]["text"], json!("Blue Vinyl"));
    assert_eq!(produced["items"][0]["appearance"], json!([]), "free text is provenance, not a mapped appearance");
}

// ── Attachment at the producer boundary ─────────────────────────────

#[test]
fn test_attach_media_block_leaves_the_raw_formats_untouched() {
    let formats = json!([{"name": "Vinyl", "qty": "2", "descriptions": {"description": ["LP", "Album"]}}]);
    let mut record = json!({"id": "1", "title": "A Release", "formats": formats.clone()});
    attach_media_block(&mut record);
    assert_eq!(record["formats"], formats, "the raw provider field stays the provenance record");
    assert_eq!(record["media"]["items"][0]["medium"], json!("vinyl_12"));
    assert_eq!(record["id"], json!("1"));
}

#[test]
fn test_attach_media_block_on_a_record_without_formats() {
    let mut record = json!({"id": "1", "title": "A Release"});
    attach_media_block(&mut record);
    assert_eq!(record["media"]["items"], json!([]));
    assert_eq!(record["media"]["families"], json!([]));
    assert_eq!(record["media"]["taxonomy_version"], json!("1"));
}

#[test]
fn test_attach_media_block_ignores_a_non_object_record() {
    let mut record = json!(["not", "a", "record"]);
    attach_media_block(&mut record);
    assert_eq!(record, json!(["not", "a", "record"]));
}

// ── The block travels through the normalizer, inside the hash ───────

async fn normalize_one(data_type: DataType, record: Value) -> DataMessage {
    let (in_sender, in_receiver) = mpsc::channel::<DataMessage>(1);
    let (out_sender, mut out_receiver) = mpsc::channel::<DataMessage>(1);
    in_sender
        .send(DataMessage { id: "1".to_string(), sha256: String::new(), data: record, raw_xml: None })
        .await
        .expect("the message is queued");
    drop(in_sender);
    message_normalizer(in_receiver, out_sender, data_type).await.expect("the normalizer runs");
    out_receiver.recv().await.expect("the normalizer emits the message")
}

/// The normalizer attaches the block to the XML-shaped release the parser emits, mapping the
/// `formats` list after it has been unwrapped and de-prefixed.
#[tokio::test]
async fn test_normalizer_attaches_the_block_to_releases() {
    let record = json!({
        "id": "1",
        "title": "A Release",
        "formats": {"format": [{"@name": "Vinyl", "@qty": "2", "descriptions": {"description": ["LP", "Album", "Reissue"]}}]}
    });
    let got = normalize_one(DataType::Releases, record).await;

    assert_eq!(got.data["media"]["items"][0]["medium"], json!("vinyl_12"));
    assert_eq!(got.data["media"]["items"][0]["qty"], json!(2));
    assert_eq!(got.data["media"]["release_kind"], json!("album"));
    assert_eq!(got.data["media"]["edition"], json!(["reissue"]));
    assert_eq!(got.data["formats"][0]["name"], json!("Vinyl"), "the normalized raw formats survive");
    assert!(!got.sha256.is_empty(), "the normalizer populates the content hash");
}

/// Only releases carry a block: the other data types have no media.
#[tokio::test]
async fn test_normalizer_attaches_no_block_to_other_types() {
    for data_type in [DataType::Artists, DataType::Labels, DataType::Masters] {
        let got = normalize_one(data_type, json!({"id": "1", "name": "Aphex Twin"})).await;
        assert!(got.data.get("media").is_none(), "{data_type:?} must not carry a media block");
    }
}

/// The block is attached before the hash, so a change the vocabulary sees but the raw bytes
/// barely show still changes the content hash consumers key change detection on.
#[tokio::test]
async fn test_hash_covers_the_media_block() {
    let release = |descriptions: Value| {
        json!({
            "id": "1",
            "title": "A Release",
            "formats": {"format": [{"@name": "Vinyl", "@qty": "1", "descriptions": {"description": descriptions}}]}
        })
    };
    let seven = normalize_one(DataType::Releases, release(json!(["7\""]))).await;
    let twelve = normalize_one(DataType::Releases, release(json!(["12\""]))).await;
    let seven_again = normalize_one(DataType::Releases, release(json!(["7\""]))).await;

    assert_ne!(seven.sha256, twelve.sha256, "a different resolved medium must change the hash");
    assert_eq!(seven.sha256, seven_again.sha256, "the same record must hash the same");
    assert_eq!(seven.data["media"]["items"][0]["medium"], json!("vinyl_7"));
    assert_eq!(twelve.data["media"]["items"][0]["medium"], json!("vinyl_12"));
}

/// Attaching the block is what makes the hash differ: the same record hashed without it
/// would collide with one whose media differ only through the vocabulary.
#[tokio::test]
async fn test_hash_differs_from_the_same_record_without_a_block() {
    let record = json!({"id": "1", "title": "A Release", "formats": {"format": [{"@name": "CD", "@qty": "1"}]}});
    let with_block = normalize_one(DataType::Releases, record.clone()).await;

    let mut without_block = with_block.data.clone();
    without_block.as_object_mut().expect("the record is an object").remove("media");
    assert_ne!(with_block.sha256, crate::types::calculate_content_hash(&without_block), "the hash must cover the media block");
}
