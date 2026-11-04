#!/usr/bin/env python3
"""Validate the small documentation header schema and emit a JSON inventory."""

import argparse
from datetime import date
import json
from pathlib import Path
import re
import sys


BASE_FIELDS = {"id", "title", "kind", "status", "implementation", "topics"}
EXTRA_FIELDS = {
    "work": {"owner", "depends", "authority", "blocked_on", "evidence", "direction"},
    "ruling": {"proposer", "decision_maker", "authority", "decided"},
    "record": {"authority", "decided"},
    "direction": {"priority", "horizon", "authority", "decided"},
}
FIELDS = BASE_FIELDS | set().union(*EXTRA_FIELDS.values())
STATUSES = {
    "design": {"draft", "accepted"},
    "reference": {"draft", "accepted"},
    "explanation": {"draft", "accepted"},
    "howto": {"draft", "accepted"},
    "record": {"draft", "accepted"},
    "direction": {"draft", "accepted"},
    "ruling": {"open", "approved", "declined", "withdrawn"},
    "work": {"planned", "approved", "active", "blocked", "done", "dropped"},
}
CHOICES = {
    "kind": set(STATUSES),
    "status": set().union(*STATUSES.values()),
    "implementation": {"planned", "partial", "implemented", "not-applicable"},
    "priority": {"high", "moderate", "low"},
    "horizon": {"now", "near", "far"},
}
IDENTIFIER = re.compile(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*\Z")


class DocumentError(Exception):
    def __init__(self, path, code, detail):
        self.path = path
        self.code = code
        self.detail = detail

    def __str__(self):
        return f"{self.path}: {self.code}: {self.detail}"


def read_document(path, root):
    lines = path.read_text(encoding="utf-8").splitlines()
    relative = str(path.relative_to(root))
    if not lines or lines[0] != "---":
        raise DocumentError(relative, "missing-header", "expected opening ---")
    try:
        end = lines.index("---", 1)
    except ValueError:
        raise DocumentError(relative, "unclosed-header", "expected closing ---")

    record = {}
    for line in lines[1:end]:
        key, separator, value = line.partition(":")
        if not separator or key not in FIELDS:
            raise DocumentError(relative, "unknown-field", key)
        if key in record:
            raise DocumentError(relative, "duplicate-field", key)
        value = value.strip()
        if not value:
            raise DocumentError(relative, "empty-field", key)
        record[key] = value

    missing = BASE_FIELDS - record.keys()
    if missing:
        raise DocumentError(relative, "missing-fields", ", ".join(sorted(missing)))
    if not IDENTIFIER.fullmatch(record["id"]):
        raise DocumentError(relative, "invalid-id", record["id"])
    for field, choices in CHOICES.items():
        if field in record and record[field] not in choices:
            raise DocumentError(relative, "invalid-choice", f"{field}={record[field]}")
    kind = record["kind"]
    status = record["status"]
    if status not in STATUSES[kind]:
        raise DocumentError(relative, "invalid-choice", f"status={status} for {kind}")
    extra = record.keys() - BASE_FIELDS - EXTRA_FIELDS.get(kind, set())
    if extra:
        raise DocumentError(relative, "unknown-field", ", ".join(sorted(extra)))
    for field in record.keys() - BASE_FIELDS - {"depends", "decided"}:
        value = record[field]
        if (value[0] in "'\"!&*{}[]>|%@`?#:," or ": " in value or " #" in value
                or value.lower() in {"null", "true", "false", "~"}
                or not any(c.isalpha() for c in value)):
            raise DocumentError(relative, "invalid-scalar", f"{field} must be plain one-line text")
    required = set()
    if kind == "work":
        required |= {"owner", "depends"}
        if status in {"approved", "active", "blocked", "done"}:
            required.add("authority")
        if status == "blocked":
            required.add("blocked_on")
        if status == "done":
            required.add("evidence")
        if status != "blocked" and "blocked_on" in record:
            raise DocumentError(relative, "stale-blocker", "blocked_on requires blocked status")
    if kind == "ruling":
        required |= {"proposer", "decision_maker"}
        if status in {"approved", "declined"}:
            required |= {"authority", "decided"}
        elif "authority" in record or "decided" in record:
            raise DocumentError(relative, "undecided-ruling", "decision metadata requires a decision")
    if kind in {"record", "direction"} and status == "accepted":
        required |= {"authority", "decided"}
    if kind == "direction":
        required |= {"priority", "horizon"}
        if record["implementation"] != "not-applicable":
            raise DocumentError(relative, "invalid-choice", "direction implementation must be not-applicable")
    missing = required - record.keys()
    if missing:
        raise DocumentError(relative, "missing-fields", ", ".join(sorted(missing)))
    if "decided" in record:
        try:
            if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", record["decided"]):
                raise ValueError("expected YYYY-MM-DD")
            date.fromisoformat(record["decided"])
        except ValueError:
            raise DocumentError(relative, "invalid-date", record["decided"]) from None
    if "depends" in record:
        value = record["depends"]
        if not value.startswith("[") or not value.endswith("]"):
            raise DocumentError(relative, "invalid-dependencies", "expected [id, repository:id] or []")
        dependencies = [] if value == "[]" else [item.strip() for item in value[1:-1].split(",")]
        for dependency in dependencies:
            parts = dependency.split(":")
            if len(parts) not in {1, 2} or not all(IDENTIFIER.fullmatch(part) for part in parts):
                raise DocumentError(relative, "invalid-dependencies", value)
        if len(dependencies) != len(set(dependencies)):
            raise DocumentError(relative, "invalid-dependencies", "duplicate dependency")
        record["depends"] = dependencies
    if "direction" in record:
        parts = record["direction"].split(":")
        if len(parts) not in {1, 2} or not all(IDENTIFIER.fullmatch(part) for part in parts):
            raise DocumentError(relative, "invalid-direction", record["direction"])

    value = record["topics"]
    if not value.startswith("[") or not value.endswith("]"):
        raise DocumentError(relative, "invalid-topics", "expected [topic, topic]")
    topics = [topic.strip() for topic in value[1:-1].split(",")]
    if not all(IDENTIFIER.fullmatch(topic) for topic in topics):
        raise DocumentError(relative, "invalid-topics", value)
    if len(topics) != len(set(topics)):
        raise DocumentError(relative, "duplicate-topic", value)
    # Restrict titles to plain YAML strings so another YAML reader agrees.
    title = record["title"]
    if title[0] in "'\"!&*{}[]>|%@`?#:," or ": " in title or " #" in title:
        raise DocumentError(relative, "invalid-title", "use a plain one-line title")
    if title.lower() in {"null", "true", "false", "~"} or not any(c.isalpha() for c in title):
        raise DocumentError(relative, "invalid-title", "use a descriptive text title")
    record["topics"] = topics
    record["path"] = relative
    return record


def inventory(root):
    directory = root / "docs"
    if not directory.is_dir():
        raise DocumentError("docs", "missing-directory", "expected documentation directory")
    records = []
    identifiers = {}
    for path in sorted(directory.rglob("*.md")):
        record = read_document(path, root)
        if record["id"] in identifiers:
            raise DocumentError(record["path"], "duplicate-id", identifiers[record["id"]])
        identifiers[record["id"]] = record["path"]
        records.append(record)
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check", "inventory"))
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    for field, choices in CHOICES.items():
        parser.add_argument(f"--{field}", choices=sorted(choices))
    parser.add_argument("--topic")
    args = parser.parse_args()
    try:
        records = inventory(args.root.resolve())
    except (DocumentError, OSError, UnicodeError) as error:
        print(error, file=sys.stderr)
        return 1

    if args.command == "check":
        print(f"Validated {len(records)} documents.")
        return 0
    for field in CHOICES:
        wanted = getattr(args, field)
        if wanted:
            records = [record for record in records if record.get(field) == wanted]
    if args.topic:
        records = [record for record in records if args.topic in record["topics"]]
    print(json.dumps(records, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
