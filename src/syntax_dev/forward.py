"""Transparent Rust CLI delegation, without mirrored commands or validation."""

import subprocess
import sys

import click


class RustGroup(click.Group):
    """Keep Python-owned commands local and forward every other argument vector."""

    def __init__(self, *args, rust_command, **kwargs):
        super().__init__(*args, **kwargs)
        self.rust_command = rust_command

    def parse_args(self, ctx, args):
        if args and args[0] not in self.commands and args[0] not in ("--help", "-h"):
            ctx.meta["rust_args"] = list(args)
            return super().parse_args(ctx, [])
        return super().parse_args(ctx, args)

    def invoke(self, ctx):
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
                f"Cannot launch Rust CLI {self.rust_command[0]}: {error}. "
                "Build the CLI explicitly or configure rust-command."
            ) from error
        if capture:
            click.echo(result.stdout, nl=False, color=True)
            click.echo(result.stderr, nl=False, err=True, color=True)
        ctx.exit(result.returncode if result.returncode >= 0 else 128 - result.returncode)
