"""Tests of metadata validation and the query interface; no YAML dependency."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import docs


HEADER = """---
id: sample
title: Sample interface
kind: design
status: draft
implementation: planned
topics: [interface, compiler]
---

# Sample interface
"""


class DocumentationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "docs").mkdir()
        self.path = self.root / "docs" / "sample.md"
        self.path.write_text(HEADER)

    def test_inventory_keeps_approval_separate_from_implementation(self):
        self.path.write_text(HEADER.replace("status: draft", "status: accepted"))
        record, = docs.inventory(self.root)
        self.assertEqual(record["status"], "accepted")
        self.assertEqual(record["implementation"], "planned")
        self.assertEqual(record["topics"], ["interface", "compiler"])

    def test_duplicate_identity_across_files_is_rejected(self):
        (self.root / "docs" / "other.md").write_text(HEADER)
        with self.assertRaises(docs.DocumentError) as caught:
            docs.inventory(self.root)
        self.assertEqual(caught.exception.code, "duplicate-id")

    def test_incomplete_and_unknown_metadata_are_rejected(self):
        cases = [
            (HEADER.replace("implementation: planned\n", ""), "missing-fields"),
            (HEADER.replace("status: draft", "status: done"), "invalid-choice"),
            (HEADER.replace("kind: design", "owner: someone\nkind: design"), "unknown-field"),
            (HEADER.replace("id: sample", "id: sample\nid: again"), "duplicate-field"),
            (HEADER.replace("[interface, compiler]", "[interface, interface]"), "duplicate-topic"),
            (HEADER.replace("[interface, compiler]", "[]"), "invalid-topics"),
            (HEADER.replace("---", "", 1), "missing-header"),
        ]
        for text, code in cases:
            with self.subTest(code=code):
                self.path.write_text(text)
                with self.assertRaises(docs.DocumentError) as caught:
                    docs.inventory(self.root)
                self.assertEqual(caught.exception.code, code)

    def test_cli_filters_inventory_and_returns_failure_for_invalid_docs(self):
        command = [sys.executable, str(Path(docs.__file__)), "inventory", "--root", str(self.root)]
        result = subprocess.run(command + ["--topic", "interface", "--status", "draft"],
                                capture_output=True, text=True, check=True)
        self.assertEqual([record["id"] for record in json.loads(result.stdout)], ["sample"])
        result = subprocess.run(command + ["--status", "accepted"],
                                capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(result.stdout), [])
        self.path.write_text("# No metadata\n")
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)


if __name__ == "__main__":
    unittest.main()
