#!/usr/bin/env python3
"""Validate documentation metadata and content, and emit a JSON inventory.

This file is the canonical validator. The workspace imports it as
cartan_dev.docs; each component vendors a byte-identical copy as
tools/docs.py, which runs standalone with only the standard library.
"""

import argparse
from dataclasses import asdict, dataclass
from datetime import date
import json
import os
from pathlib import Path
import re
import sys
from urllib.parse import unquote


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

# Path of the vendored copy inside each component repository.
VENDORED_PATH = "tools/docs.py"
WORK_LINE_LIMIT = 300

ERROR, WARNING, INFO = "error", "warning", "info"
SEVERITY = {
    "broken-link": ERROR,
    "broken-anchor": ERROR,
    "broken-external-link": WARNING,
    "broken-external-anchor": WARNING,
    "commit-sha": WARNING,
    "ignored-output": WARNING,
    "done-work": WARNING,
    "long-work": WARNING,
    "external-unavailable": INFO,
}

FENCE = re.compile(r" {0,3}(`{3,}|~{3,})")
HEADING = re.compile(r" {0,3}#{1,6}(?:[ \t]+(.*?))?(?:[ \t]+#+)?[ \t]*$")
HTML_ANCHOR = re.compile(r"<a\s[^>]*?\b(?:name|id)=[\"']([^\"']+)[\"']")
INLINE_CODE = re.compile(r"(`+)(.+?)\1")
LINK = re.compile(r"\[(?:[^\[\]]|\[[^\[\]]*\])*\]\(\s*(<[^>]*>|[^\s()]+(?:\([^\s()]*\)[^\s()]*)*)"
                  r"(?:\s+(?:\"[^\"]*\"|'[^']*'))?\s*\)")
DEFINITION = re.compile(r" {0,3}\[[^\]]+\]:\s*(<[^>]*>|\S+)")
SCHEME = re.compile(r"[A-Za-z][A-Za-z0-9+.-]*:")
# Commit-like hexadecimal: 7-40 lowercase hex characters with a digit and a letter,
# excluding decimal fractions and exponents such as 1.48416483839398e-38.
COMMIT = re.compile(r"(?<!\.)\b(?=[0-9a-f]*[0-9])(?=[0-9a-f]*[a-f])[0-9a-f]{7,40}\b(?![-+.]\d)")
OUTPUT_PATH = re.compile(r"(?:^|(?<=[\s`(\[\"'/]))(out/[^\s`()\[\]\"'<>]+)")


class DocumentError(Exception):
    def __init__(self, path, code, detail):
        self.path = path
        self.code = code
        self.detail = detail

    def __str__(self):
        return f"{self.path}: {self.code}: {self.detail}"


@dataclass(frozen=True)
class Finding:
    """A content lint fact; `severity` follows from the stable `code`."""

    code: str
    path: str
    line: int = None
    target: str = None
    count: int = None

    @property
    def severity(self):
        return SEVERITY[self.code]

    def to_json(self):
        data = {key: value for key, value in asdict(self).items() if value is not None}
        data["severity"] = self.severity
        return data


@dataclass(frozen=True)
class CopyStatus:
    """Whether one repository's copy matches the canonical file."""

    repository: str
    path: str
    state: str  # identical, different or missing


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


def prose_lines(lines):
    """Yield (number, line) outside front matter and fenced code blocks."""
    start = 0
    if lines and lines[0] == "---" and "---" in lines[1:]:
        start = lines.index("---", 1) + 1
    fence = None
    for number, line in enumerate(lines[start:], start + 1):
        match = FENCE.match(line)
        if fence is None:
            if match:
                fence = match.group(1)
            else:
                yield number, line
        elif (match and match.group(1)[0] == fence[0] and len(match.group(1)) >= len(fence)
              and not line[match.end():].strip()):
            fence = None


def slug(heading):
    """GitHub-style heading anchor, before duplicate numbering."""
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", heading)
    text = re.sub(r"<[^>]+>", "", text).strip().lower()
    return re.sub(r"[^\w\- ]", "", text).replace(" ", "-")


def anchors(lines):
    found, counts = set(), {}
    for _, line in prose_lines(lines):
        match = HEADING.match(line)
        if match:
            base = slug(match.group(1) or "")
            count = counts.get(base, 0)
            counts[base] = count + 1
            found.add(base if count == 0 else f"{base}-{count}")
        found.update(HTML_ANCHOR.findall(line))
    return found


def nested_repositories(root):
    """Submodule paths, which are separate repositories inside this checkout."""
    try:
        text = (root / ".gitmodules").read_text(encoding="utf-8")
    except OSError:
        return []
    return [Path(path) for path in re.findall(r"^\s*path\s*=\s*(.+?)\s*$", text, re.M)]


def external_root(root, target, nested):
    """Return the other repository's entry point, or None for this repository."""
    try:
        relative = target.relative_to(root)
    except ValueError:
        common = Path(os.path.commonpath([root, target]))
        parts = target.relative_to(common).parts
        return common / parts[0] if parts else common
    for path in nested:
        if relative.parts[:len(path.parts)] == path.parts:
            return root / path
    return None


def link_targets(line):
    text = INLINE_CODE.sub(lambda match: " " * len(match.group(0)), line)
    targets = [match.group(1) for match in LINK.finditer(text)]
    definition = DEFINITION.match(text)
    if definition:
        targets.append(definition.group(1))
    return [target[1:-1] if target.startswith("<") else target for target in targets]


def check_links(root, path, relative, prose, nested, cache):
    def anchors_of(file):
        if file not in cache:
            cache[file] = anchors(file.read_text(encoding="utf-8").splitlines())
        return cache[file]

    findings = []
    for number, line in prose:
        for target in link_targets(line):
            if not target or SCHEME.match(target) or target.startswith("//"):
                continue
            location, _, fragment = target.partition("#")
            location = unquote(location.partition("?")[0])
            if location.startswith("/"):
                resolved = root / location.lstrip("/")
            else:
                resolved = path.parent / location if location else path
            resolved = Path(os.path.normpath(resolved))
            other = external_root(root, resolved, nested)
            if other is not None and not (other.is_file() or other.is_dir() and any(other.iterdir())):
                findings.append(Finding("external-unavailable", relative, number, target))
                continue
            prefix = "broken-external-" if other is not None else "broken-"
            if not resolved.exists():
                findings.append(Finding(prefix + "link", relative, number, target))
            elif (fragment and resolved.suffix == ".md" and resolved.is_file()
                  and unquote(fragment) not in anchors_of(resolved)):
                findings.append(Finding(prefix + "anchor", relative, number, target))
    return findings


def lint(root, records=None):
    """Return content findings for README, top-level guidance and docs/."""
    root = Path(root).resolve()
    if records is None:
        records = inventory(root)
    work = {record["path"]: record for record in records if record["kind"] == "work"}
    nested = nested_repositories(root)
    files = [path for path in sorted(root.glob("*.md")) if path.name != "HANDOFF.md"]
    files += sorted((root / "docs").rglob("*.md"))
    findings, cache = [], {}
    for path in files:
        relative = path.relative_to(root).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()
        prose = list(prose_lines(lines))
        if relative == "README.md" or relative.startswith(("docs/records/", "docs/work/")):
            for number, line in prose:
                findings.extend(Finding("commit-sha", relative, number, sha)
                                for sha in COMMIT.findall(line))
        if relative == "README.md" or relative.startswith("docs/"):
            for number, line in prose:
                findings.extend(Finding("ignored-output", relative, number, path_text)
                                for path_text in OUTPUT_PATH.findall(line))
        record = work.get(relative)
        if record is not None:
            if record["status"] == "done":
                findings.append(Finding("done-work", relative))
            if len(lines) > WORK_LINE_LIMIT:
                findings.append(Finding("long-work", relative, count=len(lines)))
        findings.extend(check_links(root, path, relative, prose, nested, cache))
    return findings


def failed(findings, strict=False):
    failing = {ERROR, WARNING} if strict else {ERROR}
    return any(finding.severity in failing for finding in findings)


def compare_copies(reference, repositories):
    """Compare each repository's copies with canonical files.

    `reference` maps a repository-relative path to its canonical file;
    `repositories` maps a repository name to its checkout directory.
    """
    statuses = []
    for name, directory in repositories.items():
        for relative, canonical in reference.items():
            copy = Path(directory) / relative
            if not copy.is_file():
                state = "missing"
            else:
                state = "identical" if copy.read_bytes() == Path(canonical).read_bytes() else "different"
            statuses.append(CopyStatus(name, relative, state))
    return statuses


def render_finding(finding):
    """Human presentation of one finding; the only place lint prose is written."""
    location = finding.path if finding.line is None else f"{finding.path}:{finding.line}"
    code = finding.code
    if code in {"broken-link", "broken-external-link"}:
        message = f"link target does not exist: {finding.target}"
    elif code in {"broken-anchor", "broken-external-anchor"}:
        message = f"link anchor does not exist: {finding.target}"
    elif code == "external-unavailable":
        message = f"repository not present, link not checked: {finding.target}"
    elif code == "commit-sha":
        message = f"commit-like hash in prose may go stale: {finding.target}"
    elif code == "ignored-output":
        message = f"cites ignored output path: {finding.target}"
    elif code == "done-work":
        message = "work item is done; consolidate its results and delete it"
    elif code == "long-work":
        message = f"work item has {finding.count} lines (limit {WORK_LINE_LIMIT}); split or consolidate it"
    else:
        message = finding.target or ""
    return f"{finding.severity}: {location}: {code}: {message}"


def default_root():
    """The nearest ancestor of this file holding docs/, independent of cwd."""
    for directory in Path(__file__).resolve().parents:
        if (directory / "docs").is_dir():
            return directory
    return Path.cwd()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("command", choices=("check", "inventory"))
    parser.add_argument("--root", type=Path, default=None)
    parser.add_argument("--strict", action="store_true", help="check: fail on warnings too")
    parser.add_argument("--json", action="store_true", help="check: print findings as JSON")
    for field, choices in CHOICES.items():
        parser.add_argument(f"--{field}", choices=sorted(choices))
    parser.add_argument("--topic")
    args = parser.parse_args(argv)
    root = (args.root or default_root()).resolve()
    try:
        records = inventory(root)
        findings = lint(root, records) if args.command == "check" else []
    except (DocumentError, OSError, UnicodeError) as error:
        print(error, file=sys.stderr)
        return 1

    if args.command == "check":
        status = 1 if failed(findings, args.strict) else 0
        if args.json:
            report = {"documents": len(records), "findings": [f.to_json() for f in findings]}
            print(json.dumps(report, indent=2, ensure_ascii=False))
            return status
        for finding in findings:
            if finding.severity != INFO:
                print(render_finding(finding))
        counts = {severity: sum(f.severity == severity for f in findings)
                  for severity in (ERROR, WARNING, INFO)}
        print(f"Validated {len(records)} documents: {counts[ERROR]} errors, "
              f"{counts[WARNING]} warnings, {counts[INFO]} external links not checked.")
        return status
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
