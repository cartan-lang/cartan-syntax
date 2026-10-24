"""Version-one component factories and standalone entry point."""

import asyncio
from contextlib import asynccontextmanager
from html import escape
from importlib.resources import files
import json
from pathlib import Path
import tomllib
from urllib.parse import parse_qs, urlsplit, urlunsplit

import click
from markdown_it import MarkdownIt
from rich.console import Console
from starlette.applications import Starlette
from starlette.responses import HTMLResponse, JSONResponse, Response
from starlette.routing import Route

from .service import inspect


def _root(repo_root: Path) -> Path:
    root = Path(repo_root)
    if not root.is_absolute():
        raise ValueError("repo_root must be absolute")
    return root.resolve()


def _command(command: tuple[str, ...] | None) -> tuple[str, ...] | None:
    if command is not None and (
        not isinstance(command, tuple) or not command
        or not command[0]
        or not all(isinstance(token, str) and "\0" not in token for token in command)
    ):
        raise ValueError("rust_command requires an executable and literal string arguments")
    return command


def _configured_command(
    root: Path, explicit: tuple[str, ...] | None
) -> tuple[str, ...] | None:
    if explicit is not None:
        return _command(explicit)
    project = root / "pyproject.toml"
    if not project.is_file():
        return None
    try:
        settings = tomllib.loads(project.read_text(encoding="utf-8"))
        command = settings.get("tool", {}).get("cartdev", {}).get("rust-command")
    except (OSError, ValueError, AttributeError) as error:
        raise ValueError(f"Cannot read command settings in {project}: {error}") from error
    if command is None:
        return None
    if not isinstance(command, list):
        raise ValueError("tool.cartdev.rust-command must be an array of strings")
    return _command(tuple(command))


def create_cli(*, repo_root: Path, rust_command: tuple[str, ...] | None = None) -> click.Group:
    root = _root(repo_root)
    rust_command = _configured_command(root, rust_command)

    @click.group(name="syntax", invoke_without_command=True)
    @click.pass_context
    def cli(ctx):
        """Inspect syntax and browse component documentation."""
        if ctx.invoked_subcommand is None:
            click.echo(ctx.get_help())

    @cli.command("ir")
    @click.argument("source", type=click.Path(path_type=Path))
    @click.option("--json", "as_json", is_flag=True, help="Emit a version-one result envelope.")
    @click.option("--timeout", type=click.FloatRange(min=0, min_open=True), default=30.0)
    def ir(source, as_json, timeout):
        """Display the source AST using the existing Rust CLI."""
        result = asyncio.run(inspect(root, rust_command, source.resolve(), timeout))
        if as_json:
            click.echo(json.dumps(result.envelope()))
        else:
            click.echo(result.stdout, nl=False)
            click.echo(result.stderr, err=True, nl=False)
        if result.status != "ok":
            raise click.exceptions.Exit(result.exit_code if result.exit_code and result.exit_code > 0 else 1)

    @cli.command()
    @click.option("--host", default="127.0.0.1", show_default=True)
    @click.option("--port", type=click.IntRange(1, 65535), default=8000, show_default=True)
    def serve(host, port):
        """Serve documentation and explicit local diagnostics."""
        import uvicorn
        Console(stderr=True).print(f"Syntax development app: http://{host}:{port}")
        uvicorn.run(create_app(repo_root=root, rust_command=rust_command), host=host, port=port)

    return cli


def create_app(*, repo_root: Path, rust_command: tuple[str, ...] | None = None,
               navigation: tuple[tuple[str, str], ...] = ()) -> Starlette:
    root = _root(repo_root)
    rust_command = _configured_command(root, rust_command)
    markdown = MarkdownIt("commonmark", {"html": False}).enable("table")

    @asynccontextmanager
    async def lifespan(app):
        app.state.started = True
        app.state.operations = set()
        try:
            yield
        finally:
            for task in tuple(app.state.operations):
                task.cancel()
            await asyncio.gather(*app.state.operations, return_exceptions=True)
            app.state.started = False

    def route_url(request, name, **kwargs):
        # Resolve against this child router, not the aggregate's route names.
        # ASGI root_path includes both deployment and component mount prefixes.
        prefix = request.scope.get("root_path", "").rstrip("/")
        return prefix + str(app.url_path_for(name, **kwargs))

    def page(request, title, body, *, section="overview", intro="", status_code=200, show_title=True):
        def url(name, **kwargs):
            return escape(route_url(request, name, **kwargs), quote=True)
        global_links = []
        for label, href in navigation:
            current = ' aria-current="page"' if href.rstrip("/") == route_url(request, "syntax_home").rstrip("/") else ""
            global_links.append(f'<a href="{escape(href, quote=True)}"{current}>{escape(label)}</a>')
        local_links = []
        for key, label, href in (
            ("overview", "Overview", url("syntax_home")),
            ("docs", "Documentation", url("syntax_docs", path="")),
            ("diagnostics", "Diagnostics", url("syntax_diagnostics")),
        ):
            current = ' aria-current="page"' if section == key else ""
            local_links.append(f'<a href="{href}"{current}>{label}</a>')
        heading = f'<h1>{escape(title)}</h1>' if show_title else f'<p class="subtle">{escape(title)}</p>'
        description = f'<p class="subtle">{escape(intro)}</p>' if intro else ""
        html = (
            '<!doctype html><html lang="en"><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width, initial-scale=1">'
            f'<title>{escape(title)} · Cartan Syntax</title>'
            f'<link rel="stylesheet" href="{url("syntax_style")}">'
            f'<script defer src="{url("syntax_script")}"></script>'
            '</head><body><a class="skip-link" href="#main">Skip to content</a>'
            '<header class="site-header"><div class="header-inner">'
            f'<a class="brand" href="{url("syntax_home")}"><span class="brand-mark" aria-hidden="true">C</span>Cartan</a>'
            f'<nav class="global-nav" aria-label="Workspace">{"".join(global_links)}</nav>'
            '</div></header><div class="app-shell">'
            f'<nav class="local-nav" aria-label="Syntax">{"".join(local_links)}</nav>'
            '<main id="main" tabindex="-1"><div class="page-intro">'
            f'<p class="eyebrow">Syntax</p>{heading}{description}</div>{body}</main>'
            '</div></body></html>'
        )
        return HTMLResponse(html, status_code=status_code)

    async def home(request):
        docs_url = escape(route_url(request, "syntax_docs", path=""), quote=True)
        diagnostics_url = escape(route_url(request, "syntax_diagnostics"), quote=True)
        body = (
            '<div class="card-grid">'
            f'<a class="card" href="{docs_url}"><p class="eyebrow">Explore</p>'
            '<h2>Documentation</h2>'
            '<p>Read the grammar, source AST interface and component guides.</p>'
            '<span class="card-action">Open documentation <span aria-hidden="true">→</span></span></a>'
            f'<a class="card" href="{diagnostics_url}"><p class="eyebrow">Inspect</p>'
            '<h2>Diagnostics</h2>'
            '<p>Inspect a source file’s syntax tree, source spans and parse diagnostics.</p>'
            '<span class="card-action">Open diagnostics <span aria-hidden="true">→</span></span></a>'
            '</div><section class="panel"><h2>From source to structure</h2>'
            '<p>Explore concrete syntax trees and the supported source AST. Syntax inspection '
            'describes written constructs; it does not resolve names or validate execution.</p></section>'
        )
        return page(request, "Source syntax, made visible", body,
                    intro="Inspect Cartan source and explore the language’s written structure.")

    async def docs(request):
        name = request.path_params["path"]
        if not name:
            readme_url = escape(route_url(request, "syntax_docs", path="README.md"), quote=True)
            links = [f'<li><a href="{readme_url}">README</a></li>']
            docs_root = (root / "docs").resolve()
            if docs_root.is_relative_to(root):
                for path in sorted(docs_root.rglob("*.md")):
                    resolved = path.resolve()
                    if resolved.is_relative_to(docs_root):
                        relative = path.relative_to(docs_root).as_posix()
                        href = escape(route_url(request, "syntax_docs", path=relative), quote=True)
                        links.append(f'<li><a href="{href}">{escape(relative)}</a></li>')
            return page(request, "Documentation", '<section class="panel"><ul class="doc-list">' + ''.join(links) + '</ul></section>', section="docs", intro="Guides, interface contracts and maintained component references.")
        base = root if name == "README.md" else root / "docs"
        path = (base / name).resolve()
        allowed = (path == root / "README.md" if name == "README.md" else
                   path.is_relative_to((root / "docs").resolve()) and (root / "docs").resolve().is_relative_to(root))
        if not allowed or path.suffix != ".md" or not path.is_file():
            return page(request, "Document not found", '<p class="notice error" role="alert">This document is unavailable. Choose a document from the documentation index.</p>', section="docs", status_code=404)
        source = path.read_text(encoding="utf-8")
        if source.startswith("---\n") and "\n---\n" in source[4:]:
            source = source[4:].split("\n---\n", 1)[1]
        tokens = markdown.parse(source)
        for token in tokens:
            for child in token.children or ():
                attribute = "href" if child.type == "link_open" else "src" if child.type == "image" else None
                value = child.attrGet(attribute) if attribute else None
                if not value:
                    continue
                parts = urlsplit(value)
                if parts.scheme or parts.netloc or not parts.path:
                    continue
                target = (path.parent / parts.path).resolve()
                if target == root / "README.md":
                    destination = "README.md"
                elif target.is_relative_to((root / "docs").resolve()) and target.suffix == ".md":
                    destination = target.relative_to((root / "docs").resolve()).as_posix()
                else:
                    child.attrSet(attribute, "#")
                    continue
                href = route_url(request, "syntax_docs", path=destination)
                child.attrSet(attribute, urlunsplit((*urlsplit(href)[:3], parts.query, parts.fragment)))
        has_title = any(token.type == "heading_open" and token.tag == "h1" for token in tokens)
        return page(request, name, '<article class="panel prose">' + markdown.renderer.render(tokens, markdown.options, {}) + '</article>', section="docs", show_title=not has_title)

    def inspection_page(request, *, source="", result=None, error=None, status_code=200):
        action = escape(route_url(request, "syntax_inspect"), quote=True)
        body = (
            '<section class="panel"><h2>Inspect a source file</h2>'
            '<p class="subtle" id="source-help">Use a path relative to this component repository.</p>'
            f'<form method="post" action="{action}" data-inspect-form>'
            '<label for="source">Source path<input id="source" name="source" required '
            f'value="{escape(source, quote=True)}" aria-describedby="source-help" '
            'placeholder="tests/fixtures/diffusion.cart" autocomplete="off" spellcheck="false"></label>'
            '<button type="submit">Inspect source</button></form></section>'
        )
        if error:
            body += f'<div class="notice error" role="alert">{escape(error)}</div>'
        elif result is not None:
            labels = {
                "ok": "Inspection complete.",
                "failed": "The source could not be inspected. Review the diagnostics below.",
                "unavailable": "The syntax inspector is unavailable. Review the diagnostics below.",
                "timeout": "The inspection timed out. Review the diagnostics below.",
                "cancelled": "The inspection was cancelled.",
            }
            notice = "notice" if result.status == "ok" else "notice error"
            body += f'<div class="{notice}" role="status">{escape(labels[result.status])}</div>'
            body += '<section class="panel"><h2>Source AST</h2>'
            body += f'<pre class="inspection-output" tabindex="0">{escape(result.stdout or "No AST output.")}</pre>'
            if result.stderr:
                body += f'<h2>Diagnostics</h2><pre class="inspection-output" tabindex="0">{escape(result.stderr)}</pre>'
            body += '</section>'
        else:
            body += '<p class="notice" role="status">Choose a source file to begin.</p>'
        body += '<p class="notice" role="status" data-inspect-pending hidden>Inspecting source…</p>'
        return page(request, "Diagnostics", body, section="diagnostics", status_code=status_code,
                    intro="Inspect the source AST and review parser diagnostics.")

    async def diagnostics(request):
        return inspection_page(request)

    async def run_inspect(request):
        wants_json = "application/json" in request.headers.get("content-type", "")

        def invalid(message):
            if wants_json:
                return JSONResponse({"error": message}, status_code=400)
            return inspection_page(request, error=message, status_code=400)

        if wants_json:
            try:
                data = await request.json()
                source = data["source"]
            except (ValueError, KeyError, TypeError):
                return invalid("source is required")
        else:
            try:
                data = parse_qs((await request.body()).decode("utf-8"))
            except UnicodeDecodeError:
                return invalid("form must be UTF-8")
            source = data.get("source", [None])[0]
        if not isinstance(source, str) or not source:
            return invalid("source is required")
        try:
            source_path = (root / source).resolve()
        except (OSError, ValueError, RuntimeError):
            return invalid("invalid source path")
        task = asyncio.create_task(inspect(root, rust_command, source_path))
        request.app.state.operations.add(task)
        try:
            result = await task
        finally:
            request.app.state.operations.discard(task)
        if wants_json:
            return JSONResponse(result.envelope())
        return inspection_page(request, source=source, result=result)

    async def style(request):
        return Response(files("syntax_dev").joinpath("static/style.css").read_text(), media_type="text/css")

    async def script(request):
        return Response(files("syntax_dev").joinpath("static/diagnostics.js").read_text(), media_type="text/javascript")

    routes = [
        Route("/", home, name="syntax_home"),
        Route("/docs/{path:path}", docs, name="syntax_docs"),
        Route("/diagnostics/", diagnostics, name="syntax_diagnostics"),
        Route("/api/inspect", run_inspect, methods=["POST"], name="syntax_inspect"),
        Route("/static/style.css", style, name="syntax_style"),
        Route("/static/diagnostics.js", script, name="syntax_script"),
    ]
    app = Starlette(routes=routes, lifespan=lifespan)
    app.state.started = False
    app.state.operations = set()
    return app


def component():
    return {"api_version": 1, "id": "syntax", "title": "Syntax",
            "create_cli": create_cli, "create_app": create_app}


@click.command(context_settings={"ignore_unknown_options": True, "allow_extra_args": True, "help_option_names": []})
@click.option("--repo", type=click.Path(path_type=Path), help="Component repository (otherwise search invocation directory parents).")
@click.option("--rust-command", multiple=True, help="Executable/prefix token; repeat for each argument. No shell is run.")
@click.pass_context
def main(ctx, repo, rust_command):
    """Syntax development commands. Use --repo PATH for another checkout."""
    if repo is None:
        repo = next((p for p in (Path.cwd(), *Path.cwd().parents)
                     if (p / "pyproject.toml").is_file() and
                     tomllib.loads((p / "pyproject.toml").read_text()).get("project", {}).get("name") == "cartan-syntax-dev"), None)
    if repo is None and (not ctx.args or ctx.args == ["--help"]):
        cli = create_cli(repo_root=Path.cwd())
        click.echo(cli.get_help(click.Context(cli, info_name=ctx.info_name)))
        click.echo("\nStandalone options:\n  --repo PATH  Component repository.\n  --rust-command TEXT  Executable/prefix token (repeatable).")
        return
    if repo is None:
        raise click.ClickException("No syntax repository found; supply --repo PATH")
    repo = repo.resolve()
    try:
        cli = create_cli(repo_root=repo, rust_command=tuple(rust_command) or None)
    except ValueError as error:
        raise click.ClickException(str(error)) from error
    if not ctx.args or ctx.args == ["--help"]:
        click.echo(cli.get_help(click.Context(cli, info_name=ctx.info_name)))
        click.echo("\nStandalone options:\n  --repo PATH  Component repository.\n  --rust-command TEXT  Executable/prefix token (repeatable).")
        return
    cli.main(args=ctx.args, prog_name=ctx.info_name, standalone_mode=False)
