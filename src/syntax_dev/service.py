"""Bounded asynchronous delegation to the syntax CLI."""

import asyncio
import os
import signal
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Literal


Status = Literal["ok", "failed", "unavailable", "timeout", "cancelled"]


@dataclass(frozen=True)
class Result:
    status: Status
    exit_code: int | None = None
    stdout: str = ""
    stderr: str = ""
    schema_version: int = 1

    def envelope(self):
        return asdict(self)


async def inspect(repo_root: Path, rust_command: tuple[str, ...] | None,
                  source: Path, timeout: float = 30) -> Result:
    command = rust_command or (str(repo_root / "target/debug/cartan-syntax"),)
    try:
        process = await asyncio.create_subprocess_exec(
            *command, "ir", str(source), cwd=repo_root,
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
            start_new_session=True)
    except OSError as error:
        return Result("unavailable", stderr=f"{error}. Build the syntax CLI with cargo build in {repo_root}, or configure rust-command.\n")
    communication = asyncio.create_task(process.communicate())
    try:
        stdout, stderr = await asyncio.wait_for(asyncio.shield(communication), timeout)
    except (asyncio.TimeoutError, asyncio.CancelledError) as error:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        stdout, stderr = await communication
        if isinstance(error, asyncio.CancelledError):
            raise
        return Result("timeout", process.returncode,
                      stdout.decode(errors="replace"), stderr.decode(errors="replace") + f"\nSyntax inspection timed out after {timeout:g} seconds.\n")
    return Result("ok" if process.returncode == 0 else "failed", process.returncode,
                  stdout.decode(errors="replace"), stderr.decode(errors="replace"))
