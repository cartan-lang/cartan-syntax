#!/usr/bin/env python3
"""Validate the small documentation header schema and emit a JSON inventory."""

import argparse
import json
from pathlib import Path
import re
import sys


FIELDS = {"id", "title", "kind", "status", "implementation", "topics"}
CHOICES = {
    "kind": {"design", "reference", "explanation", "howto"},
    "status": {"draft", "accepted"},
    "implementation": {"planned", "partial", "implemented", "not-applicable"},
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

    missing = FIELDS - record.keys()
    if missing:
        raise DocumentError(relative, "missing-fields", ", ".join(sorted(missing)))
    if not IDENTIFIER.fullmatch(record["id"]):
        raise DocumentError(relative, "invalid-id", record["id"])
    for field, choices in CHOICES.items():
        if record[field] not in choices:
            raise DocumentError(relative, "invalid-choice", f"{field}={record[field]}")

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
            records = [record for record in records if record[field] == wanted]
    if args.topic:
        records = [record for record in records if args.topic in record["topics"]]
    print(json.dumps(records, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
