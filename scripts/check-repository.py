#!/usr/bin/env python3
"""Credential-free repository boundary and metadata checks."""

from __future__ import annotations

import json
from pathlib import Path
import re
import tomllib


ROOT = Path(__file__).resolve().parents[1]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]
require(cargo["license"] == "MIT", "Cargo package must use the approved MIT license")
require(cargo["repository"] == "https://github.com/groovemap-music/catalog-ingestion", "stale repository URL")
require(cargo["publish"] is False, "crate publication must remain disabled")

dockerfile = (ROOT / "Dockerfile").read_text(encoding="utf-8")
require(len(re.findall(r"^FROM .+@sha256:[0-9a-f]{64}(?: AS \w+)?$", dockerfile, re.MULTILINE)) == 2, "both image stages must be digest-pinned")
require("COPY Cargo.toml ./" in dockerfile and "COPY src ./src" in dockerfile, "Dockerfile still assumes a monorepo-relative root")
require(re.search(r"^ARG UID=1000$", dockerfile, re.MULTILINE) is not None, "Dockerfile must pin the default UID")
require(re.search(r"^ARG GID=1000$", dockerfile, re.MULTILINE) is not None, "Dockerfile must pin the default GID")
require("useradd -r -l -u ${UID}" in dockerfile, "runtime user must use the configured UID")
require(re.search(r"^USER \$\{UID\}:\$\{GID\}$", dockerfile, re.MULTILINE) is not None, "runtime USER must match the owned directories")

contract = json.loads((ROOT / "contracts/catalog-events/v1/contract.json").read_text(encoding="utf-8"))
require(contract["version"] == 1, "unexpected catalog contract version")
require((ROOT / "contracts/catalog-events/v1/bindings/python/catalog_contract.py").is_file(), "generated Python binding is absent")

for forbidden in (ROOT / "target", ROOT / "dist", ROOT / ".env"):
    require(not forbidden.is_file(), f"generated or local file is tracked at {forbidden.name}")

print("repository boundary and metadata checks passed")

