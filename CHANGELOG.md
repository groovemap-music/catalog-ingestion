# Changelog

All notable changes to this repository will be recorded here by Commitizen from
Conventional Commits.

## Unreleased

### Fix

- **image**: copy the vendored media taxonomy into the builder stage

## v0.2.0 (2026-09-04)

### Feat

- **contracts**: carry the media block in release fixtures and document it
- **rules**: flag Discogs format names the media taxonomy does not know
- **media**: add the Rust media mapper and attach the canonical media block
- **contracts**: fail contract-check on vendored taxonomy drift
- **contracts**: vendor the media taxonomy
- **telemetry**: export OTLP metrics from the extraction pipeline

### Fix

- **discogs**: invalidate Docker library cache
- **discogs**: seed Docker library target
- **toolchain**: bind pinned tools through mise
- **split**: freeze provider compatibility baseline

### Refactor

- **discogs**: make repository provider-exclusive
- **contracts**: generate source-owned v1 exports
- **runtime**: partition provider-owned modules

## v0.1.1 (2026-08-31)

### Fix

- **release**: use supported files-only flag
- **ci**: accept release-boundary bump states

## v0.1.0 (2026-08-31)

The `v0.1.0` workflow failed before publishing artifacts or images. The tag is
retained as an immutable record of that release attempt.
