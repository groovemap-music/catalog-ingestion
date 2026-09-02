"""Generated from contracts/catalog-events/definitions/discogs.json; do not edit."""

from __future__ import annotations

from os import getenv

CONTRACT_NAME = "groovemap.catalog-events"
CONTRACT_VERSION = 1
SOURCE = "discogs"
AMQP_EXCHANGE_TYPE = "fanout"
ENTITY_TYPES = ["artists", "labels", "masters", "releases"]
CONSUMERS = ["graphinator", "tableinator"]
EXCHANGE_PREFIX = getenv(
    "DISCOGS_EXCHANGE_PREFIX",
    "groovemap-discogs",
)
DEFAULT_EXCHANGE_NAMES = {
  "artists": "groovemap-discogs-artists",
  "labels": "groovemap-discogs-labels",
  "masters": "groovemap-discogs-masters",
  "releases": "groovemap-discogs-releases"
}
DEFAULT_QUEUE_NAMES = {
  "graphinator": {
    "artists": {
      "dead_letter_exchange": "groovemap-discogs-graphinator-artists.dlx",
      "dead_letter_queue": "groovemap-discogs-graphinator-artists.dlq",
      "name": "groovemap-discogs-graphinator-artists"
    },
    "labels": {
      "dead_letter_exchange": "groovemap-discogs-graphinator-labels.dlx",
      "dead_letter_queue": "groovemap-discogs-graphinator-labels.dlq",
      "name": "groovemap-discogs-graphinator-labels"
    },
    "masters": {
      "dead_letter_exchange": "groovemap-discogs-graphinator-masters.dlx",
      "dead_letter_queue": "groovemap-discogs-graphinator-masters.dlq",
      "name": "groovemap-discogs-graphinator-masters"
    },
    "releases": {
      "dead_letter_exchange": "groovemap-discogs-graphinator-releases.dlx",
      "dead_letter_queue": "groovemap-discogs-graphinator-releases.dlq",
      "name": "groovemap-discogs-graphinator-releases"
    }
  },
  "tableinator": {
    "artists": {
      "dead_letter_exchange": "groovemap-discogs-tableinator-artists.dlx",
      "dead_letter_queue": "groovemap-discogs-tableinator-artists.dlq",
      "name": "groovemap-discogs-tableinator-artists"
    },
    "labels": {
      "dead_letter_exchange": "groovemap-discogs-tableinator-labels.dlx",
      "dead_letter_queue": "groovemap-discogs-tableinator-labels.dlq",
      "name": "groovemap-discogs-tableinator-labels"
    },
    "masters": {
      "dead_letter_exchange": "groovemap-discogs-tableinator-masters.dlx",
      "dead_letter_queue": "groovemap-discogs-tableinator-masters.dlq",
      "name": "groovemap-discogs-tableinator-masters"
    },
    "releases": {
      "dead_letter_exchange": "groovemap-discogs-tableinator-releases.dlx",
      "dead_letter_queue": "groovemap-discogs-tableinator-releases.dlq",
      "name": "groovemap-discogs-tableinator-releases"
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
        raise ValueError(f"Unknown discogs consumer: {consumer}")
    _require_entity(entity)
    return f"{EXCHANGE_PREFIX}-{consumer}-{entity}"


def dead_letter_exchange_name(consumer: str, entity: str) -> str:
    return f"{queue_name(consumer, entity)}.dlx"


def dead_letter_queue_name(consumer: str, entity: str) -> str:
    return f"{queue_name(consumer, entity)}.dlq"


def _require_entity(entity: str) -> None:
    if entity not in ENTITY_TYPES:
        raise ValueError(f"Unknown discogs entity: {entity}")
