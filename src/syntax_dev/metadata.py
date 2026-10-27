"""Rust command discovery from version-one `--describe-cli` metadata.

Metadata exposes the Rust command tree to Click: names, descriptions and the
generated outline. It is never a second validator; Rust parses every argument.
"""

from dataclasses import dataclass
import json
import subprocess

import click

SCHEMA_VERSION = 1
ACTIONS = frozenset({
    "set", "append", "set_true", "set_false", "count",
    "help", "help_short", "help_long", "version",
})
EAGER = frozenset({"help", "help_short", "help_long", "version"})


@dataclass(frozen=True)
class MetadataError(Exception):
    """Typed discovery failure; the application formatter words it.

    `kind` is `launch`, `status`, `json`, `schema` or `collision`. `detail` holds
    the OS error, exit status, decode error, failing field path or name.
    """

    kind: str
    command: tuple[str, ...]
    detail: object = None


def discover(command: tuple[str, ...]) -> dict:
    """Run `command --describe-cli` once and return the validated root object."""
    try:
        result = subprocess.run([*command, "--describe-cli"], capture_output=True, check=False)
    except OSError as error:
        raise MetadataError("launch", command, error) from error
    if result.returncode != 0:
        raise MetadataError("status", command, result.returncode)
    try:
        root = json.loads(result.stdout)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise MetadataError("json", command, error) from error
    validate(root, command)
    return root


def validate(root: object, command: tuple[str, ...]) -> None:
    """Check the version-one shape this module reads, naming the failing path."""
    def fail(path):
        raise MetadataError("schema", command, path)

    def field(container, key, path):
        # Every version-one key is required, including the nullable ones.
        if key not in container:
            fail(f"{path}.{key}" if path else key)
        return container[key]

    def strings(value, path, nullable=False):
        if value is None and nullable:
            return
        if not isinstance(value, str):
            fail(path)

    def string_list(value, path):
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            fail(path)

    def argument(value, path):
        if not isinstance(value, dict):
            fail(path)
        strings(field(value, "id", path), f"{path}.id")
        for key in ("description", "long", "short"):
            strings(field(value, key, path), f"{path}.{key}", nullable=True)
        for key in ("aliases", "short_aliases", "value_names", "default_values", "choices"):
            string_list(field(value, key, path), f"{path}.{key}")
        position = field(value, "position", path)
        if position is not None and (type(position) is not int or position < 1):
            fail(f"{path}.position")
        arity = field(value, "arity", path)
        if not isinstance(arity, dict):
            fail(f"{path}.arity")
        minimum = field(arity, "min", f"{path}.arity")
        maximum = field(arity, "max", f"{path}.arity")
        if (type(minimum) is not int or minimum < 0
                or not (maximum is None or (type(maximum) is int and maximum >= minimum))):
            fail(f"{path}.arity")
        action = field(value, "action", path)
        if not isinstance(action, str) or action not in ACTIONS:
            fail(f"{path}.action")
        for key in ("repeatable", "required", "global"):
            if type(field(value, key, path)) is not bool:
                fail(f"{path}.{key}")

    def node(value, path):
        if not isinstance(value, dict):
            fail(path)
        strings(field(value, "name", path), f"{path}.name")
        strings(field(value, "description", path), f"{path}.description", nullable=True)
        string_list(field(value, "aliases", path), f"{path}.aliases")
        for key, check in (("arguments", argument), ("commands", node)):
            items = field(value, key, path)
            if not isinstance(items, list):
                fail(f"{path}.{key}")
            for index, item in enumerate(items):
                check(item, f"{path}.{key}[{index}]")

    # `True == 1` in Python, so the version must be exactly the integer 1.
    if (not isinstance(root, dict) or type(root.get("schema_version")) is not int
            or root["schema_version"] != SCHEMA_VERSION):
        fail("schema_version")
    strings(field(root, "program", ""), "program")
    strings(field(root, "version", ""), "version", nullable=True)
    node(field(root, "command", ""), "command")


def outline(node: dict, path: tuple[str, ...], *, root: bool = True) -> list[str]:
    """The Rust `--help` command outline lines for `node`, depth first.

    Help/version entries and inherited global arguments are omitted below the
    root, matching the outline Rust generates from the same tree.
    """
    path = (*path, node["name"]) if not root or not path else path
    about = f"  {node['description']}" if node["description"] else ""
    lines = [f"  {' '.join(path)}{about}"]
    for argument in node["arguments"]:
        if argument["action"] in EAGER or (argument["global"] and not root):
            continue
        name = argument["value_names"][0] if argument["value_names"] else argument["id"].upper()
        takes_value = argument["action"] in ("set", "append")
        if argument["position"] is not None:
            spelling = f"<{name}>" if argument["required"] else f"[{name}]"
        else:
            flag = f"--{argument['long']}" if argument["long"] else f"-{argument['short']}"
            spelling = f"{flag} <{name}>" if takes_value else flag
        if argument["repeatable"]:
            spelling += "..."
        notes = []
        if takes_value and argument["default_values"]:
            notes.append("default: " + ", ".join(argument["default_values"]))
        if takes_value and argument["choices"]:
            notes.append("choices: " + ", ".join(argument["choices"]))
        note = f" [{'; '.join(notes)}]" if notes else ""
        lines.append(f"      {spelling}  {argument['description'] or ''}{note}")
    for child in node["commands"]:
        lines.extend(outline(child, path, root=False))
    return lines


DISPLAY_ONLY = {"ignore_unknown_options": True, "allow_extra_args": True,
                "allow_interspersed_args": False}


class RustCommand(click.Command):
    """A leaf command derived from metadata. Its arguments reach Rust unparsed."""

    def __init__(self, node: dict):
        super().__init__(name=node["name"], help=node["description"], add_help_option=False,
                         context_settings=DISPLAY_ONLY)
        self.node = node


class RustCommandGroup(click.Group):
    """A metadata command with children, traversable like any Click group."""

    def __init__(self, node: dict):
        super().__init__(name=node["name"], help=node["description"], add_help_option=False,
                         context_settings=DISPLAY_ONLY)
        self.node = node
        for child in node["commands"]:
            self.add_command(rust_command(child))


def rust_command(node: dict) -> click.Command:
    """The Click object for a metadata node. Invocation still happens at the
    root group, which forwards the original argv; these objects never parse."""
    return RustCommandGroup(node) if node["commands"] else RustCommand(node)
