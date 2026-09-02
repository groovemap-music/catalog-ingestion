# Provider-split repository cutover handoff

The machine-readable handoff is
[`handoff/provider-split-cutover.json`](../handoff/provider-split-cutover.json). Its checker
anchors the implementation input to merged revision
`0ad68cc94f88bd9a13a959a61e9151f7eaba2ffd`, verifies every recorded SHA-256, and runs as
part of `just check`. The later handoff-evidence commit contains only this manifest, its
checker, documentation, and validation wiring. Infrastructure must resolve the **full reviewed
`.5` subject commit SHA from the Beadhive review gate** and use that exact commit as the seed
revision. The earlier pinned commit is only the immutable provider implementation and contract
input; it is not the repository seed.

## Source and contract ownership

Infrastructure must seed both repositories from the full reviewed handoff revision rather than
copying a mutable working directory or seeding the implementation-input parent. The checker
proves the handoff commit adds only declared evidence and validation wiring over that parent.
`discogs-ingestion` owns `src/discogs`, the Discogs definition and provider contract, and
`extraction-rules.yaml`. `musicbrainz-ingestion` owns the MusicBrainz downloader, JSONL
parser/enrichment, source loop, definition, and provider contract. Both repositories begin
with the recorded shared runtime, event envelope, release tooling, and pinned dependency graph,
then prune the other provider and adapt the composition root, generated binding, checks, image
metadata, and release metadata to their one source.

The manifest names the required seed paths and the exact repository, release-artifact, OCI
image, runtime-product, and User-Agent identity surfaces that change. The `extractor` binary
name, source exchange prefixes, v1 envelope, queue identifiers, and state-marker formats remain
compatibility interfaces and do not change during identity cutover.

## Migration safety is not final runtime policy

During migration, two producers for the **same source** must never publish to that source's
production exchanges at once. Cut over Discogs and MusicBrainz independently: stop and verify
the old producer for one source is quiescent, then enable only that source's new producer.
This guards duplicate publication; it does not impose ordering between different sources.

The final source-owned Discogs and MusicBrainz containers are required to be able to ingest
concurrently. They must have no cross-container health polling, startup or run ordering,
shared lock, or mutual exclusion. Before the MusicBrainz source-owned container is enabled in
production, remove `src/musicbrainz/combined_runtime_compat.rs`, its tests,
`DISCOGS_HEALTH_URL`, and every `wait_for_discogs_idle` call site. That module preserves the
old combined deployment only and is not a policy to reproduce in infrastructure.

## Rollback boundary

Rollback is source-local. Quiesce the new producer for the affected source, retain its data
volume and durable state marker, and only then resume the old producer from that state. Do not
run old and new instances for that same source concurrently. A Discogs rollback must not stop
MusicBrainz, or vice versa, unless an independent incident response calls for both. Exchanges,
queues, event bytes, entity vocabularies, and marker formats stay fixed across rollback.

The local rehearsals do not rename repositories, seed remote history, publish images, create
tags or releases, apply OpenTofu, or create resources. Those actions remain behind their
operator approvals.

Run `just source-characterization` to replay the focused provider boundary, source-loop,
Discogs parsing/normalization/rules, and MusicBrainz parsing/enrichment suites. The complete
Rust suite remains part of `just check`.

## Clean handoff boundary

The checker requires a clean Git status and rejects tracked credentials, keys, data dumps,
state markers, build output, authentication material, or release output. Ignored `target/`,
`dist/`, and `lcov.info` are reconstructable local outputs, never handoff inputs. The final
operator report records the handoff commit and confirms clean status after all four strict
local gates.
