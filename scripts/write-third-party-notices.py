#!/usr/bin/env python3
"""Write deterministic dependency attribution from Cargo metadata."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess


root = Path(__file__).resolve().parents[1]
metadata = json.loads(
    subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
)
workspace = set(metadata["workspace_members"])
packages = [
    {
        "name": package["name"],
        "version": package["version"],
        "license": package["license"],
        "repository": package["repository"],
    }
    for package in metadata["packages"]
    if package["id"] not in workspace
]
packages.sort(key=lambda item: (item["name"], item["version"]))
(root / "dist" / "THIRD_PARTY_NOTICES.json").write_text(
    json.dumps({"generated_from": "Cargo.lock", "packages": packages}, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

