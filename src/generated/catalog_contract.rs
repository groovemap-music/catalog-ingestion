// Generated from contracts/catalog-events/definitions/discogs.json; do not edit.

pub const CONTRACT_NAME: &str = "groovemap.catalog-events";
pub const CONTRACT_VERSION: u32 = 1;
pub const SOURCE: &str = "discogs";
pub const AMQP_EXCHANGE_TYPE: &str = "fanout";
pub const EXCHANGE_PREFIX_ENV: &str = "DISCOGS_EXCHANGE_PREFIX";
pub const DEFAULT_EXCHANGE_PREFIX: &str = "groovemap-discogs";
pub const ENTITY_TYPES: &[&str] = &["artists", "labels", "masters", "releases"];
pub const CONSUMERS: &[&str] = &["graphinator", "tableinator"];
