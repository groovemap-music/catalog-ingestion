#!/usr/bin/env python3
"""Credential-free repository boundary and metadata checks."""

from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def markdown_anchors(path: Path) -> set[str]:
    """Return GitHub-style anchors for the ordinary headings used by maintained docs."""
    anchors: set[str] = set()
    duplicates: dict[str, int] = {}
    for heading in re.findall(r"^#{1,6}\s+(.+?)\s*#*\s*$", path.read_text(encoding="utf-8"), re.MULTILINE):
        plain = re.sub(r"<[^>]+>", "", heading)
        plain = re.sub(r"[`*_~]", "", plain).strip().lower()
        slug = re.sub(r"[^\w\s-]", "", plain)
        slug = re.sub(r"\s+", "-", slug)
        duplicate = duplicates.get(slug, 0)
        duplicates[slug] = duplicate + 1
        anchors.add(slug if duplicate == 0 else f"{slug}-{duplicate}")
    return anchors


def check_local_markdown_links(index: Path) -> None:
    """Require every local README/index link and Markdown anchor to resolve."""
    text = index.read_text(encoding="utf-8")
    for raw_target in re.findall(r"(?<!!)\[[^]]+\]\(([^)\s]+)", text):
        target = raw_target.strip("<>")
        parsed = urlsplit(target)
        if parsed.scheme or parsed.netloc:
            continue

        relative_path = unquote(parsed.path)
        destination = index if not relative_path else (index.parent / relative_path).resolve()
        require(destination.is_relative_to(ROOT), f"Markdown link escapes the repository in {index.relative_to(ROOT)}: {target}")
        require(destination.exists(), f"Broken Markdown link in {index.relative_to(ROOT)}: {target}")

        if parsed.fragment:
            require(destination.is_file(), f"Markdown anchor does not target a file in {index.relative_to(ROOT)}: {target}")
            anchor = unquote(parsed.fragment).lower()
            require(anchor in markdown_anchors(destination), f"Broken Markdown anchor in {index.relative_to(ROOT)}: {target}")


def check_conceptual_diagrams() -> None:
    """Keep maintained conceptual diagrams in reviewable, source-native Mermaid."""
    diagram_docs = (
        ROOT / "README.md",
        ROOT / "docs" / "extraction.md",
        ROOT / "docs" / "extraction-rules-guide.md",
        ROOT / "docs" / "state-marker-system.md",
        ROOT / "docs" / "state-marker-periodic-updates.md",
        ROOT / "docs" / "decisions" / "0001-producer-normalization-boundary.md",
        ROOT / "docs" / "decisions" / "0002-discogs-first-musicbrainz-coordination.md",
    )
    non_mermaid_diagram_fences = {"blockdiag", "d2", "dot", "graphviz", "nomnoml", "plantuml", "puml", "seqdiag"}

    for path in diagram_docs:
        text = path.read_text(encoding="utf-8")
        require("```mermaid" in text, f"Maintained conceptual diagram must use Mermaid in {path.relative_to(ROOT)}")

    for path in (ROOT / "README.md", *(ROOT / "docs").rglob("*.md")):
        languages = {language.lower() for language in re.findall(r"^\s*```([\w+-]+)\s*$", path.read_text(encoding="utf-8"), re.MULTILINE)}
        forbidden = sorted(languages & non_mermaid_diagram_fences)
        require(not forbidden, f"Conceptual diagrams must use Mermaid in {path.relative_to(ROOT)}; found {', '.join(forbidden)}")


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

for private_planning in (
    ROOT / ".planning",
    ROOT / "docs" / "superpowers" / "plans",
    ROOT / "docs" / "superpowers" / "specs",
):
    require(
        not private_planning.exists(),
        f"private planning material must not be published at {private_planning.relative_to(ROOT)}",
    )

for documentation_index in (ROOT / "README.md", ROOT / "docs" / "README.md"):
    check_local_markdown_links(documentation_index)

check_conceptual_diagrams()

print("repository boundary and metadata checks passed")
