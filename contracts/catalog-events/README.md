# Discogs catalog event contract

This repository owns the Discogs `groovemap.catalog-events/v1` producer contract.
The maintained source is `definitions/discogs.json`; `v1/` contains the generated
manifest, Python binding, JSON fixtures, and shared event schema.

```bash
just contract
just contract-check
```

The contract preserves the `groovemap-discogs` exchange prefix and the artists, labels,
masters, and releases vocabulary. Consumer repositories promote artifacts from a
reviewed immutable commit rather than editing generated output.
