#!/usr/bin/env python3
"""Verify the immutable, source-owned repository-cutover handoff."""

from __future__ import annotations

from fnmatch import fnmatch
from hashlib import sha256
import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "handoff" / "provider-split-cutover.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def git(*args: str, text: bool = True) -> str | bytes:
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=text,
    )
    return result.stdout.strip() if text else result.stdout


def sha256_bytes(data: bytes) -> str:
    return sha256(data).hexdigest()


manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
require(manifest["schema_version"] == 1, "unsupported provider-split handoff schema")

prepared = manifest["implementation_input"]
revision = prepared["commit_sha"]
require(re.fullmatch(r"[0-9a-f]{40}", revision) is not None, "prepared revision must be a full commit SHA")
require(git("merge-base", "--is-ancestor", revision, "HEAD") == "", "prepared revision is not an ancestor of HEAD")
require(git("rev-parse", f"{revision}^{{tree}}") == prepared["tree_oid"], "prepared source tree does not match its recorded object ID")

tracked_paths = str(git("ls-tree", "-r", "--name-only", revision)).splitlines()
canonical_paths = "\n".join(sorted(tracked_paths)) + "\n"
require(len(tracked_paths) == prepared["tracked_file_count"], "prepared tracked-file count changed")
require(sha256_bytes(canonical_paths.encode()) == prepared["tracked_paths_sha256"], "prepared tracked-path digest changed")

seed_revision = manifest["seed_revision"]
require(seed_revision["evidence_revision_source"] == "review_gate_subject_commit", "seed revision must come from the review-gate subject")
require(seed_revision["requirement"] == "full-reviewed-commit-sha", "seed revision must be a full reviewed commit SHA")
actual_evidence_changes = set(str(git("diff", "--name-only", revision, "HEAD")).splitlines())
require(
    actual_evidence_changes == set(seed_revision["allowed_changes_from_implementation_input"]),
    "handoff revision contains changes outside the declared evidence and validation wiring",
)

for relative_path, expected_digest in manifest["artifact_sha256"].items():
    baseline_bytes = git("show", f"{revision}:{relative_path}", text=False)
    current_bytes = (ROOT / relative_path).read_bytes()
    require(sha256_bytes(baseline_bytes) == expected_digest, f"prepared digest mismatch: {relative_path}")
    require(sha256_bytes(current_bytes) == expected_digest, f"handoff artifact drifted after prepared revision: {relative_path}")

ownership = manifest["source_ownership"]
required_seed_paths = [
    *ownership["shared_seed_paths"],
    *ownership["discogs"]["owned_paths"],
    *ownership["musicbrainz"]["owned_paths"],
]
for relative_path in required_seed_paths:
    subprocess.run(
        ["git", "cat-file", "-e", f"{revision}:{relative_path}"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )

require(ownership["discogs"]["target_repository"] == "groovemap-music/discogs-ingestion", "unexpected Discogs repository target")
require(ownership["musicbrainz"]["target_repository"] == "groovemap-music/musicbrainz-ingestion", "unexpected MusicBrainz repository target")
musicbrainz_removals = set(ownership["musicbrainz"]["remove_provider_paths"])
require("src/musicbrainz/combined_runtime_compat.rs" in musicbrainz_removals, "MusicBrainz cutover must remove the combined-runtime compatibility module")
require("src/musicbrainz/combined_runtime_compat_tests.rs" in musicbrainz_removals, "MusicBrainz cutover must remove compatibility tests with the compatibility module")

cutover = manifest["runtime_cutover"]
require(cutover["migration_safety"]["same_source_production_exchange_dual_run"] == "forbidden", "same-source production dual-running must be forbidden")
final_runtime = cutover["final_runtime"]
require(final_runtime["cross_source_concurrency"] == "required", "final source-owned containers must be allowed to ingest concurrently")
require(
    set(final_runtime["forbidden_cross_container_dependencies"])
    == {"health_polling", "ordering", "shared_lock", "mutual_exclusion"},
    "final runtime must forbid every cross-container serialization mechanism",
)
require(
    "src/musicbrainz/combined_runtime_compat.rs" in final_runtime["remove_before_musicbrainz_production_enablement"],
    "final MusicBrainz runtime must remove combined_runtime_compat",
)

retained = manifest["retained_compatibility_identities"]
require(retained["discogs_exchange_prefix"] == "groovemap-discogs", "Discogs exchange identity changed")
require(retained["musicbrainz_exchange_prefix"] == "groovemap-musicbrainz", "MusicBrainz exchange identity changed")
require(retained["contract_name_and_version"] == "groovemap.catalog-events/v1", "catalog contract identity changed")

for evidence_path in manifest["handoff_evidence_paths"]:
    require((ROOT / evidence_path).is_file(), f"cutover evidence is absent: {evidence_path}")

tracked_now = str(git("ls-files")).splitlines()
for relative_path in tracked_now:
    for pattern in manifest["working_tree_policy"]["forbidden_tracked_globs"]:
        require(not fnmatch(relative_path, pattern), f"forbidden handoff artifact is tracked: {relative_path}")

status = str(git("status", "--porcelain=v1", "--untracked-files=all"))
require(not status, "provider-split handoff must be validated from a clean Git working tree")

print("provider-split cutover handoff checks passed")
