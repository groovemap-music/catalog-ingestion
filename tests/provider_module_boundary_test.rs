//! Structural checks for provider ownership and the removable compatibility seam.

const LEGACY_FACADE: &str = include_str!("../src/extractor.rs");
const RUNTIME: &str = include_str!("../src/runtime.rs");
const DISCOGS: &str = include_str!("../src/discogs/mod.rs");
const MUSICBRAINZ: &str = include_str!("../src/musicbrainz/mod.rs");
const COMBINED_RUNTIME_COMPAT: &str = include_str!("../src/musicbrainz/combined_runtime_compat.rs");
const MAIN: &str = include_str!("../src/main.rs");

#[test]
fn legacy_extractor_is_exports_only() {
    assert!(LEGACY_FACADE.lines().count() < 30);
    assert!(!LEGACY_FACADE.lines().any(|line| line.trim_start().starts_with("fn ")));
    assert!(!LEGACY_FACADE.lines().any(|line| line.contains("async fn ")));
}

#[test]
fn shared_runtime_has_no_provider_dependency_direction() {
    let imports_provider = RUNTIME.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("use crate::discogs")
            || line.starts_with("use crate::musicbrainz")
            || line.starts_with("use crate::parser")
            || line.starts_with("use crate::jsonl_parser")
            || line.starts_with("use crate::rules")
            || line.starts_with("use crate::normalize")
    });
    assert!(!imports_provider, "shared runtime must not import provider-owned code");
}

#[test]
fn source_modules_own_their_implementation_and_orchestration() {
    for declaration in ["pub mod downloader;", "pub mod normalize;", "pub mod parser;", "pub mod rules;"] {
        assert!(DISCOGS.contains(declaration), "Discogs boundary missing {declaration}");
    }
    for declaration in ["pub mod downloader;", "pub mod jsonl_parser;", "pub mod combined_runtime_compat;"] {
        assert!(MUSICBRAINZ.contains(declaration), "MusicBrainz boundary missing {declaration}");
    }
    assert!(MAIN.contains("discogs::run_extraction_loop("));
    assert!(MAIN.contains("musicbrainz::run_musicbrainz_loop("));
}

#[test]
fn current_coordination_is_a_musicbrainz_compatibility_seam() {
    assert!(COMBINED_RUNTIME_COMPAT.contains("Temporary coordination"));
    assert!(COMBINED_RUNTIME_COMPAT.contains("independently"));
    assert!(COMBINED_RUNTIME_COMPAT.contains("concurrently"));
    assert_eq!(MUSICBRAINZ.matches("wait_for_discogs_idle").count(), 4);
    assert!(!RUNTIME.contains("fn wait_for_discogs_idle"));
    assert!(!RUNTIME.lines().any(|line| line.trim_start().starts_with("use ") && line.contains("wait_for_discogs_idle")));
}
