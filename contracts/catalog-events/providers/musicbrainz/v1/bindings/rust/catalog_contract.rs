// Generated from contracts/catalog-events/definitions/musicbrainz.json; do not edit.

pub const CONTRACT_NAME: &str = "groovemap.catalog-events";
pub const CONTRACT_VERSION: u32 = 1;
pub const SOURCE: &str = "musicbrainz";
pub const AMQP_EXCHANGE_TYPE: &str = "fanout";
pub const EXCHANGE_PREFIX_ENV: &str = "MUSICBRAINZ_EXCHANGE_PREFIX";
pub const DEFAULT_EXCHANGE_PREFIX: &str = "groovemap-musicbrainz";
pub const ENTITY_TYPES: &[&str] = &["artists", "labels", "release-groups", "releases"];
pub const CONSUMERS: &[&str] = &["brainzgraphinator", "brainztableinator"];
pub const DEFAULT_EXCHANGE_NAMES: &[(&str, &str)] = &[
    ("artists", "groovemap-musicbrainz-artists"),
    ("labels", "groovemap-musicbrainz-labels"),
    ("release-groups", "groovemap-musicbrainz-release-groups"),
    ("releases", "groovemap-musicbrainz-releases"),
];
pub const DEFAULT_QUEUE_NAMES: &[(&str, &str, &str)] = &[
    ("brainzgraphinator", "artists", "groovemap-musicbrainz-brainzgraphinator-artists"),
    ("brainzgraphinator", "labels", "groovemap-musicbrainz-brainzgraphinator-labels"),
    ("brainzgraphinator", "release-groups", "groovemap-musicbrainz-brainzgraphinator-release-groups"),
    ("brainzgraphinator", "releases", "groovemap-musicbrainz-brainzgraphinator-releases"),
    ("brainztableinator", "artists", "groovemap-musicbrainz-brainztableinator-artists"),
    ("brainztableinator", "labels", "groovemap-musicbrainz-brainztableinator-labels"),
    ("brainztableinator", "release-groups", "groovemap-musicbrainz-brainztableinator-release-groups"),
    ("brainztableinator", "releases", "groovemap-musicbrainz-brainztableinator-releases"),
];
