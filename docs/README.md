# Catalog ingestion documentation

## Guides

- [Extraction architecture](extraction.md) — Discogs and MusicBrainz download,
  parsing, coordination, and event-publication behavior.
- [Extraction rules](extraction-rules-guide.md) — optional Discogs skip, filter,
  validation, and diagnostic-output policy.
- [State-marker system](state-marker-system.md) — version and file-level restart
  decisions, durability, and checksum provenance.
- [Periodic state-marker checkpoints](state-marker-periodic-updates.md) — checkpoint
  frequency, monitoring, and recovery guarantees.
- [Runtime identity and compatibility](runtime-identity.md) — canonical GrooveMap names
  and the internal or wire identifiers retained for compatibility.
- [Provider-split compatibility baseline](provider-split-baseline.md) — pinned contract,
  fixture, lifecycle, state, coordination, and shutdown behavior for repository separation.
- [Publication readiness](publication-readiness.md) — history-sanitation scope,
  preserved public documentation, and separate operator approval gates.
- [Catalog event contract](../contracts/catalog-events/README.md) — versioning,
  generated bindings, fixtures, and consumer promotion.

## Architecture decisions

- [Normalize at the producer boundary](decisions/0001-producer-normalization-boundary.md)
- [Coordinate MusicBrainz behind Discogs](decisions/0002-discogs-first-musicbrainz-coordination.md)

## Maintained design coverage

The public guides above preserve the durable conclusions from the earlier implementation
work without retaining raw task plans or repository-spanning proposals here.

| Design topic | Maintained documentation |
| --- | --- |
| Data-quality observation rules | [Extraction rules](extraction-rules-guide.md) |
| Skip and filter transforms | [Extraction rules](extraction-rules-guide.md) |
| Discogs and MusicBrainz producer integration | [Extraction architecture](extraction.md) |
| Automatic MusicBrainz downloads | [Extraction architecture](extraction.md#musicbrainz) |
| MusicBrainz release-group support | [Extraction architecture](extraction.md#published-entities) |
| Combined-runtime coordination and concurrent-container cutover | [Discogs-first coordination](decisions/0002-discogs-first-musicbrainz-coordination.md) |
| Producer-side normalization | [Producer normalization](decisions/0001-producer-normalization-boundary.md) |

Historical implementation plans are not part of this repository's published
documentation set.
