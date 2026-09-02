"""Regression tests for deterministic, source-owned catalog contract exports."""

from __future__ import annotations

from hashlib import sha256
import importlib.util
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "contracts" / "generate.py"
SPEC = importlib.util.spec_from_file_location("catalog_contract_generator", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class SourceContractGenerationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.combined = json.loads(GENERATOR.CONTRACT_PATH.read_text(encoding="utf-8"))
        self.definitions = {
            source: GENERATOR._load_source_definition(source)
            for source in GENERATOR.SOURCES
        }
        self.manifests = {
            source: GENERATOR._source_manifest(source, definition, self.combined)
            for source, definition in self.definitions.items()
        }

    def test_source_contracts_compose_to_the_immutable_combined_registry(self) -> None:
        composed = GENERATOR._compose_source_manifests(self.manifests)
        expected = GENERATOR._semantic_registry_from_combined(self.combined)

        self.assertEqual(composed, expected)
        GENERATOR._validate_composition(self.manifests, self.combined)

    def test_each_contract_contains_only_its_source_registry(self) -> None:
        for source, manifest in self.manifests.items():
            other_source = ({*GENERATOR.SOURCES} - {source}).pop()
            with self.subTest(source=source):
                self.assertEqual(set(manifest["sources"]), {source})
                self.assertEqual(set(manifest["fixture_payloads"]), {source})
                self.assertEqual(
                    set(manifest["consumers"]),
                    set(self.definitions[source]["consumers"]),
                )
                self.assertTrue(
                    all(details["source"] == source for details in manifest["consumers"].values())
                )
                self.assertNotIn(other_source, json.dumps(manifest, sort_keys=True))

    def test_source_bindings_do_not_expose_the_other_provider(self) -> None:
        for source, manifest in self.manifests.items():
            other_source = ({*GENERATOR.SOURCES} - {source}).pop()
            for language, content in (
                ("python", GENERATOR._render_source_python(source, manifest)),
                ("rust", GENERATOR._render_source_rust(source, manifest)),
            ):
                with self.subTest(source=source, language=language):
                    self.assertIn("do not edit", content)
                    self.assertNotIn(other_source, content)

    def test_provider_envelopes_and_completion_examples_are_legacy_bytes(self) -> None:
        rendered = GENERATOR.render_all()
        legacy_schema = (GENERATOR.CONTRACT_ROOT / "schemas" / "event.schema.json").read_text(encoding="utf-8")
        for source in GENERATOR.SOURCES:
            provider_root = GENERATOR.PROVIDERS_ROOT / source / "v1"
            with self.subTest(source=source, artifact="schema"):
                self.assertEqual(rendered[provider_root / "schemas" / "event.schema.json"], legacy_schema)
            for fixture in ("file-complete.json", "extraction-complete.json"):
                with self.subTest(source=source, artifact=fixture):
                    self.assertEqual(
                        rendered[provider_root / "fixtures" / fixture],
                        (GENERATOR.CONTRACT_ROOT / "fixtures" / fixture).read_text(encoding="utf-8"),
                    )

    def test_legacy_v1_artifact_set_and_bytes_are_pinned(self) -> None:
        GENERATOR._assert_legacy_v1_immutable()
        for relative_path, expected_digest in GENERATOR.LEGACY_V1_SHA256.items():
            with self.subTest(path=relative_path):
                self.assertEqual(
                    sha256((GENERATOR.CONTRACT_ROOT / relative_path).read_bytes()).hexdigest(),
                    expected_digest,
                )

    def test_only_source_data_fixtures_are_exported(self) -> None:
        rendered = GENERATOR.render_all()
        for source in GENERATOR.SOURCES:
            provider_root = GENERATOR.PROVIDERS_ROOT / source / "v1" / "fixtures"
            fixture_names = {
                path.name
                for path in rendered
                if path.parent == provider_root
            }
            expected_names = {
                *(f"{source}-{entity}.data.json" for entity in self.definitions[source]["source"]["entities"]),
                "extraction-complete.json",
                "file-complete.json",
            }
            with self.subTest(source=source):
                self.assertEqual(fixture_names, expected_names)


if __name__ == "__main__":
    unittest.main()
