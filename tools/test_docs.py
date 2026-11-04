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

    def test_work_lifecycle_requires_authority_blocker_and_evidence(self):
        work = HEADER.replace("kind: design", "kind: work").replace("status: draft", "status: planned")
        work = work.replace("topics:", "owner: component-agent\ndepends: [syntax:normalized-ast]\ntopics:")
        self.path.write_text(work)
        self.assertEqual(docs.inventory(self.root)[0]["depends"], ["syntax:normalized-ast"])
        for status, extra, valid in [
            ("active", "", False),
            ("active", "authority: Owner assignment\n", True),
            ("blocked", "authority: Owner assignment\n", False),
            ("blocked", "authority: Owner assignment\nblocked_on: Device access unavailable\n", True),
            ("done", "authority: Owner assignment\n", False),
            ("done", "authority: Owner assignment\nevidence: tests/check.py\n", True),
        ]:
            with self.subTest(status=status, extra=extra):
                self.path.write_text(work.replace("status: planned", f"status: {status}")
                                     .replace("topics:", extra + "topics:"))
                if valid:
                    docs.inventory(self.root)
                else:
                    with self.assertRaises(docs.DocumentError) as caught:
                        docs.inventory(self.root)
                    self.assertEqual(caught.exception.code, "missing-fields")

    def test_decisions_require_attribution_and_valid_dates(self):
        ruling = HEADER.replace("kind: design", "kind: ruling").replace("status: draft", "status: open")
        ruling = ruling.replace("topics:", "proposer: supervisor\ndecision_maker: owner\ntopics:")
        self.path.write_text(ruling)
        docs.inventory(self.root)
        approved = ruling.replace("status: open", "status: approved")
        self.path.write_text(approved)
        with self.assertRaises(docs.DocumentError) as caught:
            docs.inventory(self.root)
        self.assertEqual(caught.exception.code, "missing-fields")
        approved = approved.replace("topics:", "authority: Owner instruction\ndecided: 2026-10-23\ntopics:")
        self.path.write_text(approved)
        docs.inventory(self.root)
        self.path.write_text(approved.replace("2026-10-23", "2026-02-30"))
        with self.assertRaises(docs.DocumentError) as caught:
            docs.inventory(self.root)
        self.assertEqual(caught.exception.code, "invalid-date")

    def test_work_query_and_kind_specific_metadata(self):
        work = HEADER.replace("kind: design", "kind: work").replace("status: draft", "status: planned")
        work = work.replace("topics:", "owner: supervisor\ndepends: []\ntopics:")
        self.path.write_text(work)
        result = subprocess.run(
            [sys.executable, str(Path(docs.__file__)), "inventory", "--root", str(self.root),
             "--kind", "work", "--status", "planned"], capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(result.stdout)[0]["depends"], [])
        for invalid, code in [
            (work.replace("owner: supervisor", "owner: *supervisor"), "invalid-scalar"),
            (work.replace("depends: []", "depends: [x, x]"), "invalid-dependencies"),
            (work.replace("depends: []", "depends: [x, ]"), "invalid-dependencies"),
            (work.replace("topics:", "blocked_on: old blocker\ntopics:"), "stale-blocker"),
            (HEADER.replace("kind: design", "kind: record").replace("status: draft", "status: accepted"), "missing-fields"),
        ]:
            self.path.write_text(invalid)
            with self.assertRaises(docs.DocumentError) as caught:
                docs.inventory(self.root)
            self.assertEqual(caught.exception.code, code)

    def test_direction_metadata_and_lifecycle(self):
        direction = (HEADER.replace("kind: design", "kind: direction")
                     .replace("implementation: planned", "implementation: not-applicable")
                     .replace("topics:", "priority: high\nhorizon: near\ntopics:"))
        accepted = direction.replace("status: draft", "status: accepted").replace(
            "topics:", "authority: Owner direction\ndecided: 2026-11-04\ntopics:")
        for valid in (direction, accepted):
            self.path.write_text(valid)
            record, = docs.inventory(self.root)
            self.assertEqual((record["priority"], record["horizon"]), ("high", "near"))
        for priority in ("high", "moderate", "low"):
            for horizon in ("now", "near", "far"):
                with self.subTest(priority=priority, horizon=horizon):
                    self.path.write_text(direction.replace("priority: high", f"priority: {priority}")
                                         .replace("horizon: near", f"horizon: {horizon}"))
                    record, = docs.inventory(self.root)
                    self.assertEqual((record["priority"], record["horizon"]), (priority, horizon))
        cases = [
            (direction.replace("priority: high\n", ""), "missing-fields"),
            (direction.replace("horizon: near\n", ""), "missing-fields"),
            (direction.replace("priority: high", "priority: urgent"), "invalid-choice"),
            (direction.replace("horizon: near", "horizon: next"), "invalid-choice"),
            (direction.replace("implementation: not-applicable", "implementation: planned"), "invalid-choice"),
            (direction.replace("implementation: not-applicable", "implementation: partial"), "invalid-choice"),
            (direction.replace("implementation: not-applicable", "implementation: implemented"), "invalid-choice"),
            (direction.replace("status: draft", "status: approved"), "invalid-choice"),
            (accepted.replace("authority: Owner direction\n", ""), "missing-fields"),
            (accepted.replace("decided: 2026-11-04\n", ""), "missing-fields"),
            (accepted.replace("2026-11-04", "2026-02-30"), "invalid-date"),
            (HEADER.replace("topics:", "priority: high\ntopics:"), "unknown-field"),
            (HEADER.replace("topics:", "horizon: now\ntopics:"), "unknown-field"),
            (direction.replace("topics:", "direction: other\ntopics:"), "unknown-field"),
        ]
        for text, code in cases:
            with self.subTest(text=text):
                self.path.write_text(text)
                with self.assertRaises(docs.DocumentError) as caught:
                    docs.inventory(self.root)
                self.assertEqual(caught.exception.code, code)

    def test_work_direction_reference_and_mixed_inventory_filters(self):
        work = (HEADER.replace("kind: design", "kind: work")
                .replace("status: draft", "status: planned")
                .replace("topics:", "owner: supervisor\ndepends: []\ntopics:"))
        for reference in ("demand-planning", "cartan-lang:demand-planning"):
            self.path.write_text(work.replace("topics:", f"direction: {reference}\ntopics:"))
            self.assertEqual(docs.inventory(self.root)[0]["direction"], reference)
        for reference in ("bad id", "root:a:b", "bad/id", "root:"):
            self.path.write_text(work.replace("topics:", f"direction: {reference}\ntopics:"))
            with self.assertRaises(docs.DocumentError) as caught:
                docs.inventory(self.root)
            self.assertEqual(caught.exception.code, "invalid-direction")
        for reference, code in [("", "empty-field"), ("[]", "invalid-scalar"),
                                ("[demand-planning]", "invalid-scalar")]:
            with self.subTest(reference=reference):
                self.path.write_text(work.replace("topics:", f"direction: {reference}\ntopics:"))
                with self.assertRaises(docs.DocumentError) as caught:
                    docs.inventory(self.root)
                self.assertEqual(caught.exception.code, code)
        self.path.write_text(HEADER.replace("topics:", "direction: demand-planning\ntopics:"))
        with self.assertRaises(docs.DocumentError) as caught:
            docs.inventory(self.root)
        self.assertEqual(caught.exception.code, "unknown-field")
        self.path.write_text(work)
        other = (HEADER.replace("id: sample", "id: direction-sample")
                 .replace("kind: design", "kind: direction")
                 .replace("implementation: planned", "implementation: not-applicable")
                 .replace("topics:", "priority: moderate\nhorizon: far\ntopics:"))
        (self.root / "docs" / "direction.md").write_text(other)
        command = [sys.executable, str(Path(docs.__file__)), "inventory", "--root", str(self.root)]
        for args, expected in [(["--priority", "moderate", "--horizon", "far"], ["direction-sample"]),
                               (["--horizon", "now"], []), (["--kind", "work"], ["sample"])]:
            result = subprocess.run(command + args, capture_output=True, text=True, check=True)
            self.assertEqual([r["id"] for r in json.loads(result.stdout)], expected)


if __name__ == "__main__":
    unittest.main()
