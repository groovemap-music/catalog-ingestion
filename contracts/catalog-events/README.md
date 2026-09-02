# Catalog event contracts

See the repository [documentation index](../../docs/README.md) for extraction behavior and
producer design records.

`catalog-ingestion` owns the versioned wire contract in this directory. Version `v1`
defines the event envelopes, entity vocabulary, exchange and queue naming, extraction
rules, and deterministic fixtures consumed by the Python services.

The immutable combined contract remains under `v1/`. Maintained provider definitions live in
`definitions/discogs.json` and `definitions/musicbrainz.json`; they generate independently
promotable exports beneath `providers/<source>/v1/`. Each export contains only that source's
entity vocabulary, consumers, default exchange and queue identities, data fixtures, and
Rust/Python bindings. Both exports carry a byte-identical copy of the v1 event schema.

Run the generator from the repository root whenever a source definition changes:

```bash
mise exec -- python contracts/generate.py
```

The command preserves the existing combined Rust constants, Python binding, and fixtures and
writes the provider exports inside this repository. Generated bindings contain a provenance
header and must not be edited directly. CI verifies regeneration is byte-for-byte clean, rejects
extra generated provider files, and proves that composing the two provider registries reproduces
the immutable combined v1 registry. Consumer repositories copy or package a reviewed provider
export from an immutable `catalog-ingestion` commit; this generator never writes across repository
boundaries.

`file-complete.json` and `extraction-complete.json` are shared-envelope examples. Each provider
export owns a byte-identical copy as its provider-scoped completion example; data fixtures remain
strictly source-specific.

Contract versions describe the distribution containing an event; the v1 envelope is
kept unchanged for compatibility and therefore does not add an on-wire version field.
Breaking changes require a new sibling version directory and a coordinated producer /
consumer rollout. Additive entity fields remain compatible because data events permit
source-specific fields beyond the stable `type`, `id`, and `sha256` envelope.

The extraction policy remains `extraction-rules.yaml`; it is part of the
catalog-ingestion release and is not copied into consumers.
