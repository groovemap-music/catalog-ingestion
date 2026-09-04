//! Canonical media block (ADR 0007).
//!
//! Discogs describes a release's media as a bag of untyped strings: a format name, a
//! quantity, optional free text, and a nested description list that mixes medium facts
//! (size, speed, channels, codec) with release facts (kind, edition, packaging). Every
//! consumer that needed one of them re-derived it differently.
//!
//! This module maps that bag onto the provider-neutral vocabulary vendored at
//! `contracts/catalog-events/vocab/media-taxonomy.json` and produces the canonical `media`
//! block the producer attaches to every `releases` event, once, at the normalization
//! boundary, so the content hash covers it and every downstream store reads one shape.
//!
//! The vocabulary — never this code — decides routing, so a new upstream format name is a
//! re-vendoring, not a code change. Values the vocabulary does not know are preserved under
//! `unmapped` and never dropped.
//!
//! Behaviour is fixed by the conformance fixtures in `tests/fixtures/media/`, which the
//! design repository's reference mapper (`scripts/media-mapper.mjs`) also has to satisfy.
//! The two implementations must agree exactly; change this file only alongside those
//! fixtures.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

/// The vendored vocabulary, compiled in. `contracts/generate.py --check` fails the build
/// gate when these bytes drift from the digest recorded beside them, so parsing it is
/// infallible in practice.
const TAXONOMY_JSON: &str = include_str!("../../contracts/catalog-events/vocab/media-taxonomy.json");

// ── The vendored vocabulary ─────────────────────────────────────────

/// A media family: the closed top-level grouping (`vinyl`, `optical`, `digital`, …).
///
/// `resolve` lets a family that a source names without a medium (Discogs `Vinyl`) pick a
/// medium from an item attribute it derived from the descriptions (a `12"` descriptor picks
/// `vinyl_12`).
#[derive(Debug, Deserialize)]
struct FamilyDefinition {
    id: String,
    #[serde(default)]
    resolve: Option<ResolveRule>,
}

/// How a family resolves to a medium: look the stringified value of `attribute` up in `map`.
#[derive(Debug, Deserialize)]
struct ResolveRule {
    attribute: String,
    map: HashMap<String, String>,
}

/// A canonical medium, prefixed by its family (`vinyl_12`, `optical_cd`, `digital_file`).
///
/// `defaults` fill attributes the source never stated — 78 RPM for shellac, 12 inches for a
/// 12" medium.
#[derive(Debug, Deserialize)]
struct MediumDefinition {
    id: String,
    family: String,
    #[serde(default)]
    defaults: Map<String, Value>,
}

/// What a Discogs format *name* means: a medium, a family, a container, or a release flag.
///
/// The vocabulary guarantees these are mutually exclusive — a container or a flag entry
/// never carries a medium, and a variant never appears without one.
#[derive(Debug, Default, Deserialize)]
struct FormatEntry {
    #[serde(default)]
    family: Option<String>,
    #[serde(default)]
    medium: Option<String>,
    #[serde(default)]
    variant: Option<String>,
    #[serde(default)]
    container: Option<String>,
    #[serde(default)]
    flag: Option<String>,
}

/// What a Discogs *description* means: exactly one target, or `ignore`.
#[derive(Debug, Deserialize)]
struct DescriptionRule {
    target: String,
    #[serde(default)]
    value: Value,
}

#[derive(Debug, Deserialize)]
struct DiscogsVocabulary {
    formats: HashMap<String, FormatEntry>,
    descriptions: HashMap<String, DescriptionRule>,
}

/// The document as vendored. Sections this producer does not consume (`values`,
/// `musicbrainz`, `license`) are ignored rather than rejected.
#[derive(Debug, Deserialize)]
struct TaxonomyDocument {
    taxonomy_version: String,
    families: Vec<FamilyDefinition>,
    media: Vec<MediumDefinition>,
    discogs: DiscogsVocabulary,
}

/// The vocabulary indexed for lookup, parsed once per process.
#[derive(Debug)]
struct Taxonomy {
    taxonomy_version: String,
    families: HashMap<String, FamilyDefinition>,
    media: HashMap<String, MediumDefinition>,
    discogs: DiscogsVocabulary,
}

impl From<TaxonomyDocument> for Taxonomy {
    fn from(document: TaxonomyDocument) -> Self {
        Taxonomy {
            taxonomy_version: document.taxonomy_version,
            families: document.families.into_iter().map(|family| (family.id.clone(), family)).collect(),
            media: document.media.into_iter().map(|medium| (medium.id.clone(), medium)).collect(),
            discogs: document.discogs,
        }
    }
}

fn taxonomy() -> &'static Taxonomy {
    static TAXONOMY: OnceLock<Taxonomy> = OnceLock::new();
    TAXONOMY.get_or_init(|| {
        let document: TaxonomyDocument =
            serde_json::from_str(TAXONOMY_JSON).expect("the vendored media taxonomy is valid JSON in the expected shape");
        Taxonomy::from(document)
    })
}

// ── The canonical block ─────────────────────────────────────────────

/// The provider fields exactly as received, kept as the provenance record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemSource {
    pub provider: String,
    pub name: Option<String>,
    pub descriptions: Vec<String>,
    pub text: Option<String>,
}

/// One entry per source medium entry, in source order.
///
/// `position` and `track_count` are MusicBrainz-only and always `null` here; they are part
/// of the shape so a block means the same thing whichever producer wrote it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MediaItem {
    pub family: Option<String>,
    pub medium: Option<String>,
    pub qty: u64,
    pub size_inches: Option<Number>,
    pub speed_rpm: Option<Number>,
    pub channels: Option<String>,
    pub codec: Option<String>,
    pub variants: Vec<String>,
    pub appearance: Vec<String>,
    pub position: Option<i64>,
    pub track_count: Option<i64>,
    pub source: ItemSource,
}

/// Raw values the vocabulary did not recognise. Sorted and de-duplicated, never dropped, so
/// coverage is measurable from the published events.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Unmapped {
    pub formats: Vec<String>,
    pub descriptions: Vec<String>,
}

/// The canonical `media` block attached to every `releases` event.
///
/// Every field is always present: `null` or an empty list when unknown. Lists other than
/// `items` and `source.descriptions` are sorted and de-duplicated, so two implementations
/// serialise byte-identical output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MediaBlock {
    pub taxonomy_version: String,
    pub items: Vec<MediaItem>,
    pub families: Vec<String>,
    pub release_kind: Option<String>,
    pub traits: Vec<String>,
    pub edition: Vec<String>,
    pub packaging: Option<String>,
    pub container: Option<String>,
    pub flags: Vec<String>,
    pub unmapped: Unmapped,
}

impl MediaItem {
    fn new(name: Option<String>, descriptions: Vec<String>, text: Option<String>) -> Self {
        MediaItem {
            family: None,
            medium: None,
            qty: 1,
            size_inches: None,
            speed_rpm: None,
            channels: None,
            codec: None,
            variants: Vec::new(),
            appearance: Vec::new(),
            position: None,
            track_count: None,
            source: ItemSource { provider: "discogs".to_string(), name, descriptions, text },
        }
    }

    /// Set a medium attribute only when the source has not already stated it. Both the
    /// first-value-wins description rule and the medium defaults use this.
    fn fill_attribute(&mut self, attribute: &str, value: &Value) {
        match attribute {
            "size_inches" => fill(&mut self.size_inches, value.as_number().cloned()),
            "speed_rpm" => fill(&mut self.speed_rpm, value.as_number().cloned()),
            "channels" => fill(&mut self.channels, value.as_str().map(str::to_string)),
            "codec" => fill(&mut self.codec, value.as_str().map(str::to_string)),
            _ => {}
        }
    }

    /// The stringified attribute a family resolves a medium by, or `None` when unset.
    fn attribute_key(&self, attribute: &str) -> Option<String> {
        match attribute {
            "size_inches" => self.size_inches.as_ref().map(Number::to_string),
            "speed_rpm" => self.speed_rpm.as_ref().map(Number::to_string),
            "channels" => self.channels.clone(),
            "codec" => self.codec.clone(),
            _ => None,
        }
    }
}

impl MediaBlock {
    fn empty(taxonomy: &Taxonomy) -> Self {
        MediaBlock {
            taxonomy_version: taxonomy.taxonomy_version.clone(),
            items: Vec::new(),
            families: Vec::new(),
            release_kind: None,
            traits: Vec::new(),
            edition: Vec::new(),
            packaging: None,
            container: None,
            flags: Vec::new(),
            unmapped: Unmapped::default(),
        }
    }
}

/// Item attributes a description rule may target directly.
const ITEM_ATTRIBUTES: [&str; 4] = ["size_inches", "speed_rpm", "channels", "codec"];

/// Fill a slot only when it is still unset — the first value the source states wins, and a
/// medium default never overwrites it.
fn fill<T>(slot: &mut Option<T>, value: Option<T>) {
    if slot.is_none()
        && let Some(value) = value
    {
        *slot = Some(value);
    }
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

// ── Input handling ──────────────────────────────────────────────────

/// Flatten the description field into a plain list of strings.
///
/// The normalized dump shape wraps them (`{"description": [...]}`, or `{"description": ".."}`
/// for a single one) while the Discogs API returns a flat array; both flatten to the same
/// list, and anything else flattens to nothing.
fn flatten_descriptions(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect(),
        Some(Value::String(text)) => vec![text.clone()],
        Some(Value::Object(map)) => match map.get("description") {
            Some(nested) => flatten_descriptions(Some(nested)),
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Read `qty`, which the dump carries as a string. Anything that is not a whole number of at
/// least one counts as a single unit.
fn parse_qty(value: Option<&Value>) -> u64 {
    let text = match value {
        None | Some(Value::Null) => "1".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(other) => other.to_string(),
    };
    parse_leading_integer(&text).filter(|qty| *qty >= 1).map_or(1, |qty| qty as u64)
}

/// Read the leading integer of a string, ignoring any trailing garbage, so a Discogs `qty`
/// like `"2 discs"` still counts as two.
fn parse_leading_integer(text: &str) -> Option<i64> {
    let trimmed = text.trim_start();
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let magnitude = digits.parse::<i64>().ok()?;
    Some(if negative { -magnitude } else { magnitude })
}

// ── The mapper ──────────────────────────────────────────────────────

/// Apply a format-name entry, returning the item when the entry names a medium.
///
/// A container (`Box Set`) or flag (`All Media`) entry is a release-level fact and produces
/// no item at all — the set's real media arrive as sibling format entries.
fn apply_format_entry(block: &mut MediaBlock, entry: &FormatEntry, mut item: MediaItem) -> Option<MediaItem> {
    if let Some(container) = &entry.container {
        block.container = Some(container.clone());
    }
    if let Some(flag) = &entry.flag {
        block.flags.push(flag.clone());
    }
    if entry.family.is_none() && entry.medium.is_none() {
        return None;
    }
    if let Some(medium) = &entry.medium {
        item.medium = Some(medium.clone());
    }
    if let Some(family) = &entry.family {
        item.family = Some(family.clone());
    }
    if let Some(variant) = &entry.variant {
        item.variants.push(variant.clone());
    }
    Some(item)
}

/// Resolve the item's medium and family, fill the medium's defaults, and order its lists.
fn finish_item(mut item: MediaItem, taxonomy: &Taxonomy) -> MediaItem {
    if let Some(medium_id) = item.medium.clone()
        && item.family.is_none()
        && let Some(medium) = taxonomy.media.get(&medium_id)
    {
        item.family = Some(medium.family.clone());
    }
    if item.medium.is_none()
        && let Some(family_id) = item.family.clone()
    {
        let resolved = taxonomy.families.get(&family_id).and_then(|family| family.resolve.as_ref()).and_then(|resolve| {
            let key = item.attribute_key(&resolve.attribute)?;
            resolve.map.get(&key).cloned()
        });
        item.medium = Some(resolved.unwrap_or_else(|| format!("{family_id}_unspecified")));
    }
    if let Some(medium_id) = item.medium.clone()
        && let Some(medium) = taxonomy.media.get(&medium_id)
    {
        for (attribute, value) in medium.defaults.clone() {
            item.fill_attribute(&attribute, &value);
        }
    }
    item.variants = sorted_unique(item.variants);
    item.appearance = sorted_unique(item.appearance);
    item
}

/// Route one description onto its single target: an item attribute, an item list, a
/// release-level list, a scalar release fact, or nowhere.
fn apply_description(block: &mut MediaBlock, item: Option<&mut MediaItem>, rule: &DescriptionRule) {
    let target = rule.target.as_str();
    if target == "ignore" {
        return;
    }
    if ITEM_ATTRIBUTES.contains(&target) {
        if let Some(item) = item {
            item.fill_attribute(target, &rule.value);
        }
        return;
    }
    let Some(value) = rule.value.as_str() else {
        return;
    };
    match target {
        "variant" => {
            if let Some(item) = item {
                item.variants.push(value.to_string());
            }
        }
        "appearance" => {
            if let Some(item) = item {
                item.appearance.push(value.to_string());
            }
        }
        "trait" => block.traits.push(value.to_string()),
        "edition" => block.edition.push(value.to_string()),
        "flag" => block.flags.push(value.to_string()),
        // The first value wins for a scalar release fact.
        "release_kind" => fill(&mut block.release_kind, Some(value.to_string())),
        "packaging" => fill(&mut block.packaging, Some(value.to_string())),
        "container" => fill(&mut block.container, Some(value.to_string())),
        _ => {}
    }
}

fn finish_block(mut block: MediaBlock) -> MediaBlock {
    block.families = sorted_unique(block.items.iter().filter_map(|item| item.family.clone()).collect());
    block.traits = sorted_unique(block.traits);
    block.edition = sorted_unique(block.edition);
    block.flags = sorted_unique(block.flags);
    block.unmapped.formats = sorted_unique(block.unmapped.formats);
    block.unmapped.descriptions = sorted_unique(block.unmapped.descriptions);
    block
}

/// Map a normalized Discogs `formats` list onto the canonical media block.
///
/// A missing or non-array `formats` (a release the dump gave no format at all) yields the
/// empty block rather than no block, so every release carries the same shape.
pub fn map_discogs_formats(formats: Option<&Value>) -> MediaBlock {
    let taxonomy = taxonomy();
    let mut block = MediaBlock::empty(taxonomy);
    let entries: &[Value] = formats.and_then(Value::as_array).map_or(&[], Vec::as_slice);

    for format in entries {
        // The dump occasionally yields a bare string or null where a format object belongs.
        let Some(format) = format.as_object() else {
            continue;
        };
        let name = format.get("name").and_then(Value::as_str).map(str::to_string);
        let descriptions = flatten_descriptions(format.get("descriptions"));
        let text = format.get("text").and_then(Value::as_str).map(str::to_string);

        let entry = name.as_deref().and_then(|name| taxonomy.discogs.formats.get(name));
        let mut item = match entry {
            None => {
                if let Some(name) = &name {
                    block.unmapped.formats.push(name.clone());
                }
                None
            }
            Some(entry) => {
                let item = MediaItem::new(name.clone(), descriptions.clone(), text.clone());
                apply_format_entry(&mut block, entry, item).map(|mut item| {
                    item.qty = parse_qty(format.get("qty"));
                    item
                })
            }
        };

        // Release-level descriptors still apply when the entry produced no item — an
        // unknown format name or a Box Set can still carry the edition and the kind.
        for description in &descriptions {
            match taxonomy.discogs.descriptions.get(description) {
                None => block.unmapped.descriptions.push(description.clone()),
                Some(rule) => apply_description(&mut block, item.as_mut(), rule),
            }
        }

        if let Some(item) = item {
            let finished = finish_item(item, taxonomy);
            block.items.push(finished);
        }
    }

    finish_block(block)
}

/// Attach the canonical `media` block to a normalized `releases` record, in place.
///
/// The raw `formats` list is left untouched: it stays the provenance record. Callers run
/// this after normalization and before the content hash, so the hash covers the block.
pub fn attach_media_block(record: &mut Value) {
    let Some(map) = record.as_object_mut() else {
        return;
    };
    let block = map_discogs_formats(map.get("formats"));
    let Ok(value) = serde_json::to_value(&block) else {
        return;
    };
    map.insert("media".to_string(), value);
}

#[cfg(test)]
#[path = "tests/media_tests.rs"]
mod tests;
