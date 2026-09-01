// Generated from contracts/catalog-events/definitions/discogs.json; do not edit.

pub const CONTRACT_NAME: &str = "groovemap.catalog-events";
pub const CONTRACT_VERSION: u32 = 1;
pub const SOURCE: &str = "discogs";
pub const AMQP_EXCHANGE_TYPE: &str = "fanout";
pub const EXCHANGE_PREFIX_ENV: &str = "DISCOGS_EXCHANGE_PREFIX";
pub const DEFAULT_EXCHANGE_PREFIX: &str = "groovemap-discogs";
pub const ENTITY_TYPES: &[&str] = &["artists", "labels", "masters", "releases"];
pub const CONSUMERS: &[&str] = &["graphinator", "tableinator"];
pub const DEFAULT_EXCHANGE_NAMES: &[(&str, &str)] = &[
    ("artists", "groovemap-discogs-artists"),
    ("labels", "groovemap-discogs-labels"),
    ("masters", "groovemap-discogs-masters"),
    ("releases", "groovemap-discogs-releases"),
];
pub const DEFAULT_QUEUE_NAMES: &[(&str, &str, &str)] = &[
    ("graphinator", "artists", "groovemap-discogs-graphinator-artists"),
    ("graphinator", "labels", "groovemap-discogs-graphinator-labels"),
    ("graphinator", "masters", "groovemap-discogs-graphinator-masters"),
    ("graphinator", "releases", "groovemap-discogs-graphinator-releases"),
    ("tableinator", "artists", "groovemap-discogs-tableinator-artists"),
    ("tableinator", "labels", "groovemap-discogs-tableinator-labels"),
    ("tableinator", "masters", "groovemap-discogs-tableinator-masters"),
    ("tableinator", "releases", "groovemap-discogs-tableinator-releases"),
];
