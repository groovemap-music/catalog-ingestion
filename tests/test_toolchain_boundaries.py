"""Keep repository-pinned non-Rust tools behind explicit Mise boundaries."""

from __future__ import annotations

import re
import unittest
from pathlib import Path

from scripts import check_bump_preview


ROOT = Path(__file__).resolve().parents[1]
REQUIRED_TOOL = re.compile(r"(?<![\w./-])(python(?:3(?:\.\d+)*)?|uvx?|gitleaks)(?=\s|$)")
MISE_BOUNDARY = re.compile(r"\bmise\s+exec\s+--\s*$")


class ToolchainBoundaryTests(unittest.TestCase):
    """Reject ambient Python, UV, or Gitleaks fallback in executable shell sources."""

    def test_just_and_shell_invocations_cross_mise_boundary(self) -> None:
        shell_sources = (
            *ROOT.rglob("Justfile"),
            *ROOT.rglob("*.just"),
            *(ROOT / "scripts").rglob("*.sh"),
        )

        for path in shell_sources:
            for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
                if line.lstrip().startswith("#"):
                    continue
                for invocation in REQUIRED_TOOL.finditer(line):
                    with self.subTest(path=path.relative_to(ROOT), line=line_number, tool=invocation.group()):
                        self.assertRegex(
                            line[: invocation.start()],
                            MISE_BOUNDARY,
                            "repository-pinned tool invocation must follow `mise exec --`",
                        )

    def test_bump_preview_runs_uvx_through_mise(self) -> None:
        self.assertEqual(check_bump_preview.COMMAND[:4], ("mise", "exec", "--", "uvx"))


if __name__ == "__main__":
    unittest.main()
