"""Rust commands exposed from metadata and invoked with the original argv."""

import subprocess
import sys

import click

from .metadata import MetadataError, discover, outline, rust_command

HELP_SHORT, HELP_FULL = "-h", "--help"


def present(error: MetadataError) -> str:
    """The application's wording for Rust discovery and launch failures."""
    program = error.command[0]
    if error.kind == "launch":
        return (f"Cannot launch Rust CLI {program}: {error.detail}. "
                "Build the CLI explicitly or configure rust-command.")
    if error.kind == "status":
        return f"Rust CLI {program} --describe-cli exited with status {error.detail}."
    if error.kind == "json":
        return f"Rust CLI {program} --describe-cli printed invalid JSON: {error.detail}."
    if error.kind == "schema":
        return (f"Rust CLI {program} --describe-cli metadata is incompatible at "
                f"{error.detail}; schema version 1 is required.")
    if error.kind == "collision":
        return f"Rust command {error.detail!r} collides with a Python command."
    raise AssertionError(f"unhandled metadata error kind {error.kind}")


class RustGroup(click.Group):
    """Python-owned commands plus the Rust commands its metadata describes.

    Discovery runs at most once, only when the Rust tree is needed. Any argv
    not addressed to a Python command or to this group's help flags reaches
    Rust unchanged, so Rust remains the parser and validator.
    """

    def __init__(self, *args, rust_command, **kwargs):
        super().__init__(*args, **kwargs)
        self.rust_command = rust_command
        self._metadata = None
        # Shown in help only: parse_args handles these flags before Click does.
        self.params[:0] = [
            click.Option([HELP_SHORT], is_flag=True, expose_value=False, help="Show concise help."),
            click.Option([HELP_FULL], is_flag=True, expose_value=False,
                         help="Show help with the full command outline."),
        ]

    def metadata(self) -> dict:
        if self._metadata is None:
            try:
                root = discover(self.rust_command)
                for node in root["command"]["commands"]:
                    if node["name"] in self.commands:
                        raise MetadataError("collision", self.rust_command, node["name"])
                self._metadata = root
            except MetadataError as error:
                self._metadata = error
        if isinstance(self._metadata, MetadataError):
            raise self._metadata
        return self._metadata

    def rust_commands(self) -> dict[str, click.Command]:
        return {node["name"]: rust_command(node) for node in self.metadata()["command"]["commands"]}

    def list_commands(self, ctx):
        try:
            rust = list(self.rust_commands())
        except MetadataError:
            rust = []
        return sorted({*self.commands, *rust})

    def get_command(self, ctx, name):
        if name in self.commands:
            return self.commands[name]
        try:
            return self.rust_commands().get(name)
        except MetadataError:
            return None

    def help_text(self, ctx, *, full: bool) -> str:
        """Concise help, or help with the full outline, both from the trees."""
        text = self.get_help(ctx)
        try:
            root = self.metadata()
        except MetadataError as error:
            return f"{text}\n\nRust commands unavailable: {present(error)}"
        if not full:
            return text
        lines = ["Command outline:", *outline(root["command"], (ctx.command_path,))]
        for name in sorted(self.commands):
            command = self.commands[name]
            if command.hidden:
                continue
            lines.append(f"  {ctx.command_path} {name}  {command.get_short_help_str(limit=200)}")
            for param in command.params:
                if not isinstance(param, click.Option) or param.hidden:
                    continue
                flag = max(param.opts, key=len)
                metavar = param.metavar or (param.name or "value").upper()
                spelling = flag if param.is_flag else f"{flag} <{metavar}>"
                notes = []
                if not param.is_flag and param.default is not None:
                    notes.append(f"default: {param.default}")
                if isinstance(param.type, click.Choice):
                    notes.append("choices: " + ", ".join(param.type.choices))
                note = f" [{'; '.join(notes)}]" if notes else ""
                lines.append(f"      {spelling}  {param.help or ''}{note}")
        return f"{text}\n\n" + "\n".join(lines)

    def parse_args(self, ctx, args):
        if args and args[0] in (HELP_SHORT, HELP_FULL):
            ctx.meta["help"] = args[0]
            return super().parse_args(ctx, [])
        if args and args[0] not in self.commands:
            ctx.meta["rust_args"] = list(args)
            return super().parse_args(ctx, [])
        return super().parse_args(ctx, args)

    def invoke(self, ctx):
        if "help" in ctx.meta:
            click.echo(self.help_text(ctx, full=ctx.meta["help"] == HELP_FULL))
            ctx.exit(0)
        if "rust_args" not in ctx.meta:
            return super().invoke(ctx)
        try:
            # Real terminals/pipes remain attached: no buffering, recoloring, path
            # rewriting or cwd change. In-process Click callers may use streams
            # without file descriptors; capture only in that case.
            try:
                sys.stdout.fileno()
                sys.stderr.fileno()
                capture = False
            except (AttributeError, OSError, ValueError):
                capture = True
            result = subprocess.run(
                [*self.rust_command, *ctx.meta["rust_args"]],
                capture_output=capture,
                check=False,
            )
        except OSError as error:
            raise click.ClickException(
                present(MetadataError("launch", self.rust_command, error))
            ) from error
        if capture:
            click.echo(result.stdout, nl=False, color=True)
            click.echo(result.stderr, nl=False, err=True, color=True)
        ctx.exit(result.returncode if result.returncode >= 0 else 128 - result.returncode)
