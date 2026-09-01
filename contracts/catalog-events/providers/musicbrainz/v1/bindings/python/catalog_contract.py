"""Generated from contracts/catalog-events/definitions/musicbrainz.json; do not edit."""

from __future__ import annotations

from os import getenv

CONTRACT_NAME = "groovemap.catalog-events"
CONTRACT_VERSION = 1
SOURCE = "musicbrainz"
AMQP_EXCHANGE_TYPE = "fanout"
ENTITY_TYPES = ["artists", "labels", "release-groups", "releases"]
CONSUMERS = ["brainzgraphinator", "brainztableinator"]
EXCHANGE_PREFIX = getenv(
    "MUSICBRAINZ_EXCHANGE_PREFIX",
    "groovemap-musicbrainz",
)
DEFAULT_EXCHANGE_NAMES = {
  "artists": "groovemap-musicbrainz-artists",
  "labels": "groovemap-musicbrainz-labels",
  "release-groups": "groovemap-musicbrainz-release-groups",
  "releases": "groovemap-musicbrainz-releases"
}
DEFAULT_QUEUE_NAMES = {
  "brainzgraphinator": {
    "artists": {
      "dead_letter_exchange": "groovemap-musicbrainz-brainzgraphinator-artists.dlx",
      "dead_letter_queue": "groovemap-musicbrainz-brainzgraphinator-artists.dlq",
      "name": "groovemap-musicbrainz-brainzgraphinator-artists"
    },
    "labels": {
      "dead_letter_exchange": "groovemap-musicbrainz-brainzgraphinator-labels.dlx",
      "dead_letter_queue": "groovemap-musicbrainz-brainzgraphinator-labels.dlq",
      "name": "groovemap-musicbrainz-brainzgraphinator-labels"
    },
    "release-groups": {
      "dead_letter_exchange": "groovemap-musicbrainz-brainzgraphinator-release-groups.dlx",
      "dead_letter_queue": "groovemap-musicbrainz-brainzgraphinator-release-groups.dlq",
      "name": "groovemap-musicbrainz-brainzgraphinator-release-groups"
    },
    "releases": {
      "dead_letter_exchange": "groovemap-musicbrainz-brainzgraphinator-releases.dlx",
      "dead_letter_queue": "groovemap-musicbrainz-brainzgraphinator-releases.dlq",
      "name": "groovemap-musicbrainz-brainzgraphinator-releases"
    }
  },
  "brainztableinator": {
    "artists": {
      "dead_letter_exchange": "groovemap-musicbrainz-brainztableinator-artists.dlx",
      "dead_letter_queue": "groovemap-musicbrainz-brainztableinator-artists.dlq",
      "name": "groovemap-musicbrainz-brainztableinator-artists"
    },
    "labels": {
      "dead_letter_exchange": "groovemap-musicbrainz-brainztableinator-labels.dlx",
      "dead_letter_queue": "groovemap-musicbrainz-brainztableinator-labels.dlq",
      "name": "groovemap-musicbrainz-brainztableinator-labels"
    },
    "release-groups": {
      "dead_letter_exchange": "groovemap-musicbrainz-brainztableinator-release-groups.dlx",
      "dead_letter_queue": "groovemap-musicbrainz-brainztableinator-release-groups.dlq",
      "name": "groovemap-musicbrainz-brainztableinator-release-groups"
    },
    "releases": {
      "dead_letter_exchange": "groovemap-musicbrainz-brainztableinator-releases.dlx",
      "dead_letter_queue": "groovemap-musicbrainz-brainztableinator-releases.dlq",
      "name": "groovemap-musicbrainz-brainztableinator-releases"
    }
  }
}


def exchange_name(entity: str) -> str:
    """Build this source's environment-aware exchange name."""
    _require_entity(entity)
    return f"{EXCHANGE_PREFIX}-{entity}"


def queue_name(consumer: str, entity: str) -> str:
    """Build this source's registered consumer queue name."""
    if consumer not in CONSUMERS:
        raise ValueError(f"Unknown musicbrainz consumer: {consumer}")
    _require_entity(entity)
    return f"{EXCHANGE_PREFIX}-{consumer}-{entity}"


def dead_letter_exchange_name(consumer: str, entity: str) -> str:
    return f"{queue_name(consumer, entity)}.dlx"


def dead_letter_queue_name(consumer: str, entity: str) -> str:
    return f"{queue_name(consumer, entity)}.dlq"


def _require_entity(entity: str) -> None:
    if entity not in ENTITY_TYPES:
        raise ValueError(f"Unknown musicbrainz entity: {entity}")
