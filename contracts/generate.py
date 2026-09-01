"""Generate repository-owned Rust/Python artifacts and contract fixtures."""

from __future__ import annotations

import argparse
from hashlib import sha256
import json
from pathlib import Path
import sys
from typing import Any


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
CATALOG_EVENTS_ROOT = Path(__file__).resolve().parent / "catalog-events"
CONTRACT_ROOT = CATALOG_EVENTS_ROOT / "v1"
CONTRACT_PATH = CONTRACT_ROOT / "contract.json"
DEFINITIONS_ROOT = CATALOG_EVENTS_ROOT / "definitions"
PROVIDERS_ROOT = CATALOG_EVENTS_ROOT / "providers"
SOURCES = ("discogs", "musicbrainz")

# v1 is already consumed outside this repository. These hashes make its byte-level
# immutability independent of whichever rendering logic is added for future exports.
LEGACY_V1_SHA256 = {
    "bindings/python/catalog_contract.py": "0fe43d0ddbe0e36098271f5478122e9d49dd272b4ff7a927af02ee9b98bb4587",
    "contract.json": "cfb094491b2a29ab5b3ba0078476387cd29c881535464cdecc82dcbb6d5fed03",
    "fixtures/discogs-artists.data.json": "a9fe5ffd6cee9d8363e7fd919cf8407e7be0af5f22d05acdefbd9e39d90daf68",
    "fixtures/discogs-labels.data.json": "c3158577a5c21328f7bd043d047a053fa2dd40d713a1af521aaf9a9a10adeaea",
    "fixtures/discogs-masters.data.json": "d59c4aad6bae01e06d4c841e7a5003e0a34a1f2ff832af8064776b1b82f8186a",
    "fixtures/discogs-releases.data.json": "7ee129232ba5f22e43ab5250e7dc62cde3e6bc703da2f5c35348d904183b5ee8",
    "fixtures/extraction-complete.json": "06c8faabe2adcc21e2bbdd56f1eb3a4ad658a458db66966c351900df9ac422cd",
    "fixtures/file-complete.json": "8124657556b1aa9f5e650ac6570369adf495ac42beff10a719bc3c27332e73af",
    "fixtures/musicbrainz-artists.data.json": "247e703d2a3455cba10460d720ceff06f35f993f1cb79b89baa4d756587bf9ba",
    "fixtures/musicbrainz-labels.data.json": "323cd6be31f9ae38f40ca7f3373d8e0b0c88c75fcdf2635ffb4b340f38160ce2",
    "fixtures/musicbrainz-release-groups.data.json": "4e73990d3e2c3aa0582f27482457dbd00fee6aff808e8647a82adc538baa8c2f",
    "fixtures/musicbrainz-releases.data.json": "bb839624ac0f49e8a361bb4e007e36a9b0623483c2901b9fb05bf3da4999406a",
    "schemas/event.schema.json": "e89b24364448dd0496f96d1dc8b8d46198e358ac12dbd48d250dd6ef7f2967f0",
}


def _render_python(contract: dict[str, Any], *, lines_after_imports: int = 0) -> str:
    sources = contract["sources"]
    consumers = contract["consumers"]
    rendered_consumers = (
        "{\n" + "".join(f'    {json.dumps(name)}: {{"source": {json.dumps(item["source"])}}},\n' for name, item in sorted(consumers.items())) + "}"
    )
    import_spacing = "\n" * lines_after_imports
    return f'''"""Generated from contracts/catalog-events/v1/contract.json; do not edit."""

from __future__ import annotations

from os import getenv
{import_spacing}
CONTRACT_NAME = {json.dumps(contract["contract"])}
CONTRACT_VERSION = {contract["version"]}
AMQP_EXCHANGE_TYPE = {json.dumps(contract["exchange"]["kind"])}
DISCOGS_DATA_TYPES = {json.dumps(sources["discogs"]["entities"])}
MUSICBRAINZ_DATA_TYPES = {json.dumps(sources["musicbrainz"]["entities"])}
DISCOGS_EXCHANGE_PREFIX = getenv(
    {json.dumps(sources["discogs"]["exchange_prefix_env"])},
    {json.dumps(sources["discogs"]["default_exchange_prefix"])},
)
MUSICBRAINZ_EXCHANGE_PREFIX = getenv(
    {json.dumps(sources["musicbrainz"]["exchange_prefix_env"])},
    {json.dumps(sources["musicbrainz"]["default_exchange_prefix"])},
)
CONSUMER_SOURCES = {rendered_consumers}

# Compatibility names used by the current services. They are generated from the
# producer-owned contract rather than independently declared by consumers.
DATA_TYPES = DISCOGS_DATA_TYPES
AMQP_QUEUE_PREFIX_GRAPHINATOR = f"{{DISCOGS_EXCHANGE_PREFIX}}-graphinator"
AMQP_QUEUE_PREFIX_TABLEINATOR = f"{{DISCOGS_EXCHANGE_PREFIX}}-tableinator"
AMQP_QUEUE_PREFIX_BRAINZGRAPHINATOR = f"{{MUSICBRAINZ_EXCHANGE_PREFIX}}-brainzgraphinator"
AMQP_QUEUE_PREFIX_BRAINZTABLEINATOR = f"{{MUSICBRAINZ_EXCHANGE_PREFIX}}-brainztableinator"


def entity_types(source: str) -> list[str]:
    """Return the entity vocabulary for a catalog source."""
    if source == "discogs":
        return DISCOGS_DATA_TYPES
    if source == "musicbrainz":
        return MUSICBRAINZ_DATA_TYPES
    raise ValueError(f"Unknown catalog source: {{source}}")


def exchange_prefix(source: str) -> str:
    """Return the environment-aware exchange prefix for a source."""
    if source == "discogs":
        return DISCOGS_EXCHANGE_PREFIX
    if source == "musicbrainz":
        return MUSICBRAINZ_EXCHANGE_PREFIX
    raise ValueError(f"Unknown catalog source: {{source}}")


def exchange_name(source: str, entity: str) -> str:
    """Build a producer-owned exchange name."""
    _require_entity(source, entity)
    return f"{{exchange_prefix(source)}}-{{entity}}"


def queue_name(consumer: str, entity: str) -> str:
    """Build a registered consumer queue name."""
    try:
        source = CONSUMER_SOURCES[consumer]["source"]
    except KeyError as exc:
        raise ValueError(f"Unknown catalog consumer: {{consumer}}") from exc
    _require_entity(source, entity)
    return f"{{exchange_prefix(source)}}-{{consumer}}-{{entity}}"


def dead_letter_exchange_name(consumer: str, entity: str) -> str:
    """Build the dead-letter exchange name for a consumer queue."""
    return f"{{queue_name(consumer, entity)}}.dlx"


def dead_letter_queue_name(consumer: str, entity: str) -> str:
    """Build the dead-letter queue name for a consumer queue."""
    return f"{{queue_name(consumer, entity)}}.dlq"


def _require_entity(source: str, entity: str) -> None:
    if entity not in entity_types(source):
        raise ValueError(f"Unknown {{source}} entity: {{entity}}")
'''


def _render_rust(contract: dict[str, Any]) -> str:
    sources = contract["sources"]
    discogs = ", ".join(json.dumps(item) for item in sources["discogs"]["entities"])
    musicbrainz = ", ".join(json.dumps(item) for item in sources["musicbrainz"]["entities"])
    return f"""// Generated from contracts/catalog-events/v1/contract.json; do not edit.

pub const CONTRACT_NAME: &str = {json.dumps(contract["contract"])};
pub const CONTRACT_VERSION: u32 = {contract["version"]};
pub const AMQP_EXCHANGE_TYPE: &str = {json.dumps(contract["exchange"]["kind"])};
pub const DEFAULT_DISCOGS_EXCHANGE_PREFIX: &str = {json.dumps(sources["discogs"]["default_exchange_prefix"])};
pub const DEFAULT_MUSICBRAINZ_EXCHANGE_PREFIX: &str = {json.dumps(sources["musicbrainz"]["default_exchange_prefix"])};
pub const DISCOGS_ENTITY_TYPES: &[&str] = &[{discogs}];
pub const MUSICBRAINZ_ENTITY_TYPES: &[&str] = &[{musicbrainz}];
"""


def _json_text(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def _load_source_definition(source: str) -> dict[str, Any]:
    path = DEFINITIONS_ROOT / f"{source}.json"
    return json.loads(path.read_text(encoding="utf-8"))


def _source_consumers(contract: dict[str, Any], source: str) -> dict[str, dict[str, str]]:
    return {
        consumer: details
        for consumer, details in contract["consumers"].items()
        if details["source"] == source
    }


def _validate_source_definition(source: str, definition: dict[str, Any], contract: dict[str, Any]) -> None:
    expected = {
        "consumers": sorted(_source_consumers(contract, source)),
        "fixture_payloads": contract["fixture_payloads"][source],
        "source": {"name": source, **contract["sources"][source]},
    }
    if definition != expected:
        raise ValueError(f"{source} source definition does not match immutable combined v1 registry")

    other_sources = set(SOURCES) - {source}
    rendered = json.dumps(definition, sort_keys=True)
    leaked = sorted(other_source for other_source in other_sources if other_source in rendered)
    if leaked:
        raise ValueError(f"{source} source definition contains cross-source material: {', '.join(leaked)}")


def _runtime_registry(source: str, definition: dict[str, Any]) -> dict[str, Any]:
    source_definition = definition["source"]
    prefix = source_definition["default_exchange_prefix"]
    entities = source_definition["entities"]
    exchanges = {entity: f"{prefix}-{entity}" for entity in entities}
    queues: dict[str, dict[str, dict[str, str]]] = {}
    for consumer in definition["consumers"]:
        queues[consumer] = {}
        for entity in entities:
            queue = f"{prefix}-{consumer}-{entity}"
            queues[consumer][entity] = {
                "dead_letter_exchange": f"{queue}.dlx",
                "dead_letter_queue": f"{queue}.dlq",
                "name": queue,
            }
    return {"exchanges": exchanges, "queues": queues, "source": source}


def _source_manifest(source: str, definition: dict[str, Any], contract: dict[str, Any]) -> dict[str, Any]:
    source_definition = dict(definition["source"])
    source_definition.pop("name")
    return {
        "$schema": contract["$schema"],
        "consumers": {consumer: {"source": source} for consumer in definition["consumers"]},
        "contract": contract["contract"],
        "event_schema": contract["event_schema"],
        "event_schema_sha256": LEGACY_V1_SHA256["schemas/event.schema.json"],
        "exchange": contract["exchange"],
        "fixture_payloads": {source: definition["fixture_payloads"]},
        "queue": contract["queue"],
        "runtime_identifiers": _runtime_registry(source, definition),
        "sources": {source: source_definition},
        "version": contract["version"],
    }


def _semantic_registry_from_combined(contract: dict[str, Any]) -> dict[str, Any]:
    return {
        "consumers": contract["consumers"],
        "contract": contract["contract"],
        "event_schema": contract["event_schema"],
        "event_schema_sha256": LEGACY_V1_SHA256["schemas/event.schema.json"],
        "exchange": contract["exchange"],
        "fixture_payloads": contract["fixture_payloads"],
        "queue": contract["queue"],
        "runtime_identifiers": {
            source: _runtime_registry(source, _load_source_definition(source))
            for source in SOURCES
        },
        "sources": contract["sources"],
        "version": contract["version"],
    }


def _compose_source_manifests(manifests: dict[str, dict[str, Any]]) -> dict[str, Any]:
    first = manifests[SOURCES[0]]
    composed: dict[str, Any] = {
        "consumers": {},
        "contract": first["contract"],
        "event_schema": first["event_schema"],
        "event_schema_sha256": first["event_schema_sha256"],
        "exchange": first["exchange"],
        "fixture_payloads": {},
        "queue": first["queue"],
        "runtime_identifiers": {},
        "sources": {},
        "version": first["version"],
    }
    shared_keys = ("contract", "event_schema", "event_schema_sha256", "exchange", "queue", "version")
    for source, manifest in manifests.items():
        for key in shared_keys:
            if manifest[key] != first[key]:
                raise ValueError(f"{source} source contract has a divergent v1 {key}")
        composed["consumers"].update(manifest["consumers"])
        composed["fixture_payloads"].update(manifest["fixture_payloads"])
        composed["runtime_identifiers"][source] = manifest["runtime_identifiers"]
        composed["sources"].update(manifest["sources"])
    return composed


def _validate_composition(manifests: dict[str, dict[str, Any]], contract: dict[str, Any]) -> None:
    expected = _semantic_registry_from_combined(contract)
    actual = _compose_source_manifests(manifests)
    if actual != expected:
        raise ValueError("source contracts do not compose to the immutable combined v1 semantic registry")


def _render_source_python(source: str, manifest: dict[str, Any]) -> str:
    source_definition = manifest["sources"][source]
    runtime = manifest["runtime_identifiers"]
    return f'''"""Generated from contracts/catalog-events/definitions/{source}.json; do not edit."""

from __future__ import annotations

from os import getenv

CONTRACT_NAME = {json.dumps(manifest["contract"])}
CONTRACT_VERSION = {manifest["version"]}
SOURCE = {json.dumps(source)}
AMQP_EXCHANGE_TYPE = {json.dumps(manifest["exchange"]["kind"])}
ENTITY_TYPES = {json.dumps(source_definition["entities"])}
CONSUMERS = {json.dumps(sorted(manifest["consumers"]))}
EXCHANGE_PREFIX = getenv(
    {json.dumps(source_definition["exchange_prefix_env"])},
    {json.dumps(source_definition["default_exchange_prefix"])},
)
DEFAULT_EXCHANGE_NAMES = {json.dumps(runtime["exchanges"], indent=2, sort_keys=True)}
DEFAULT_QUEUE_NAMES = {json.dumps(runtime["queues"], indent=2, sort_keys=True)}


def exchange_name(entity: str) -> str:
    """Build this source's environment-aware exchange name."""
    _require_entity(entity)
    return f"{{EXCHANGE_PREFIX}}-{{entity}}"


def queue_name(consumer: str, entity: str) -> str:
    """Build this source's registered consumer queue name."""
    if consumer not in CONSUMERS:
        raise ValueError(f"Unknown {source} consumer: {{consumer}}")
    _require_entity(entity)
    return f"{{EXCHANGE_PREFIX}}-{{consumer}}-{{entity}}"


def dead_letter_exchange_name(consumer: str, entity: str) -> str:
    return f"{{queue_name(consumer, entity)}}.dlx"


def dead_letter_queue_name(consumer: str, entity: str) -> str:
    return f"{{queue_name(consumer, entity)}}.dlq"


def _require_entity(entity: str) -> None:
    if entity not in ENTITY_TYPES:
        raise ValueError(f"Unknown {source} entity: {{entity}}")
'''


def _render_source_rust(source: str, manifest: dict[str, Any]) -> str:
    source_definition = manifest["sources"][source]
    entities = ", ".join(json.dumps(item) for item in source_definition["entities"])
    consumers = ", ".join(json.dumps(item) for item in sorted(manifest["consumers"]))
    exchanges = ",\n    ".join(
        f"({json.dumps(entity)}, {json.dumps(name)})"
        for entity, name in manifest["runtime_identifiers"]["exchanges"].items()
    )
    queues = ",\n    ".join(
        f"({json.dumps(consumer)}, {json.dumps(entity)}, {json.dumps(details['name'])})"
        for consumer, entities_by_consumer in manifest["runtime_identifiers"]["queues"].items()
        for entity, details in entities_by_consumer.items()
    )
    return f"""// Generated from contracts/catalog-events/definitions/{source}.json; do not edit.

pub const CONTRACT_NAME: &str = {json.dumps(manifest["contract"])};
pub const CONTRACT_VERSION: u32 = {manifest["version"]};
pub const SOURCE: &str = {json.dumps(source)};
pub const AMQP_EXCHANGE_TYPE: &str = {json.dumps(manifest["exchange"]["kind"])};
pub const EXCHANGE_PREFIX_ENV: &str = {json.dumps(source_definition["exchange_prefix_env"])};
pub const DEFAULT_EXCHANGE_PREFIX: &str = {json.dumps(source_definition["default_exchange_prefix"])};
pub const ENTITY_TYPES: &[&str] = &[{entities}];
pub const CONSUMERS: &[&str] = &[{consumers}];
pub const DEFAULT_EXCHANGE_NAMES: &[(&str, &str)] = &[
    {exchanges},
];
pub const DEFAULT_QUEUE_NAMES: &[(&str, &str, &str)] = &[
    {queues},
];
"""


def _render_fixtures(contract: dict[str, Any]) -> dict[Path, str]:
    rendered: dict[Path, str] = {}
    for source, entities in contract["fixture_payloads"].items():
        for entity, payload in entities.items():
            event = {
                "type": "data",
                "id": f"contract-{source}-{entity}",
                "sha256": "",
                **payload,
            }
            rendered[CONTRACT_ROOT / "fixtures" / f"{source}-{entity}.data.json"] = json.dumps(event, indent=2, sort_keys=True) + "\n"
    rendered[CONTRACT_ROOT / "fixtures" / "file-complete.json"] = (
        json.dumps(
            {
                "data_type": "artists",
                "file": "contract-artists.xml.gz",
                "timestamp": "2000-01-01T00:00:00Z",
                "total_processed": 1,
                "type": "file_complete",
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    rendered[CONTRACT_ROOT / "fixtures" / "extraction-complete.json"] = (
        json.dumps(
            {
                "record_counts": {"artists": 1},
                "started_at": "2000-01-01T00:00:00Z",
                "timestamp": "2000-01-01T00:00:01Z",
                "type": "extraction_complete",
                "version": "contract-fixture",
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    return rendered


def _assert_legacy_v1_immutable() -> None:
    actual_paths = {
        path.relative_to(CONTRACT_ROOT).as_posix()
        for path in CONTRACT_ROOT.rglob("*")
        if path.is_file()
    }
    expected_paths = set(LEGACY_V1_SHA256)
    if actual_paths != expected_paths:
        raise ValueError("immutable combined v1 artifact set has changed")
    for relative_path, expected_digest in LEGACY_V1_SHA256.items():
        actual_digest = sha256((CONTRACT_ROOT / relative_path).read_bytes()).hexdigest()
        if actual_digest != expected_digest:
            raise ValueError(f"immutable combined v1 artifact changed: {relative_path}")


def _render_provider_artifacts(
    source: str,
    definition: dict[str, Any],
    manifest: dict[str, Any],
    combined_fixtures: dict[Path, str],
) -> dict[Path, str]:
    provider_root = PROVIDERS_ROOT / source / "v1"
    rendered = {
        provider_root / "bindings" / "python" / "catalog_contract.py": _render_source_python(source, manifest),
        provider_root / "bindings" / "rust" / "catalog_contract.rs": _render_source_rust(source, manifest),
        provider_root / "contract.json": _json_text(manifest),
        provider_root / "schemas" / "event.schema.json": (CONTRACT_ROOT / "schemas" / "event.schema.json").read_text(encoding="utf-8"),
    }
    provider_fixture_names = {
        *(f"{source}-{entity}.data.json" for entity in definition["source"]["entities"]),
        "extraction-complete.json",
        "file-complete.json",
    }
    for legacy_path, content in combined_fixtures.items():
        if legacy_path.name in provider_fixture_names:
            rendered[provider_root / "fixtures" / legacy_path.name] = content
    return rendered


def render_all() -> dict[Path, str]:
    """Return every generated path and its deterministic content."""
    _assert_legacy_v1_immutable()
    contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
    combined_fixtures = _render_fixtures(contract)
    rendered = {
        REPOSITORY_ROOT / "src" / "generated" / "catalog_contract.rs": _render_rust(contract),
        CONTRACT_ROOT / "bindings" / "python" / "catalog_contract.py": _render_python(contract),
    }
    rendered.update(combined_fixtures)
    definitions: dict[str, dict[str, Any]] = {}
    manifests: dict[str, dict[str, Any]] = {}
    for source in SOURCES:
        definition = _load_source_definition(source)
        _validate_source_definition(source, definition, contract)
        definitions[source] = definition
        manifests[source] = _source_manifest(source, definition, contract)
    _validate_composition(manifests, contract)
    for source in SOURCES:
        rendered.update(_render_provider_artifacts(source, definitions[source], manifests[source], combined_fixtures))
    return rendered


def main() -> int:
    """Generate artifacts, or verify that committed output is current."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if generated output differs")
    args = parser.parse_args()
    stale: list[Path] = []
    try:
        rendered = render_all()
    except (KeyError, TypeError, ValueError) as exc:
        sys.stderr.write(f"invalid catalog contract: {exc}\n")
        return 1
    for path, content in rendered.items():
        if args.check:
            if not path.exists() or path.read_text(encoding="utf-8") != content:
                stale.append(path.relative_to(REPOSITORY_ROOT))
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    if args.check and PROVIDERS_ROOT.exists():
        expected_provider_paths = {path for path in rendered if path.is_relative_to(PROVIDERS_ROOT)}
        actual_provider_paths = {path for path in PROVIDERS_ROOT.rglob("*") if path.is_file()}
        stale.extend(sorted(path.relative_to(REPOSITORY_ROOT) for path in actual_provider_paths - expected_provider_paths))
    if stale:
        sys.stderr.write("stale catalog contract artifacts:\n")
        sys.stderr.write("".join(f"  {path}\n" for path in stale))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
