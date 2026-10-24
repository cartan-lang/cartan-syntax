import asyncio
import json
from pathlib import Path
import sys

import click
from click.testing import CliRunner
import pytest
from starlette.applications import Starlette
from starlette.routing import Mount
from starlette.testclient import TestClient

from syntax_dev.application import component, create_app, create_cli, main
from syntax_dev.service import inspect


@pytest.fixture
def repo(tmp_path):
    (tmp_path / 'docs').mkdir()
    (tmp_path / 'README.md').write_text('[guide](docs/guide.md)')
    (tmp_path / 'docs/guide.md').write_text('# Guide\n<script>bad()</script>\n[Readme](../README.md)')
    return tmp_path


def fake(code):
    return (sys.executable, '-c', code)


def test_factories_cli(repo):
    assert set(component()) == {'api_version', 'id', 'title', 'create_cli', 'create_app'}
    cli = create_cli(repo_root=repo)
    runner = CliRunner()
    assert runner.invoke(cli).exit_code == 0
    outer = click.Group('cartdev')
    outer.add_command(cli)
    assert runner.invoke(outer, ['syntax']).exit_code == 0
    source = repo / 'x.cart'
    cli = create_cli(repo_root=repo, rust_command=fake('import sys; print(sys.argv[-1]); print("err", file=sys.stderr)'))
    result = runner.invoke(cli, ['ir', str(source)])
    assert result.exit_code == 0
    assert str(source) in result.stdout
    assert result.stderr == 'err\n'
    failed = create_cli(repo_root=repo, rust_command=fake('import sys; print("bad",file=sys.stderr); sys.exit(7)'))
    result = runner.invoke(failed, ['ir', '--json', str(source)])
    assert result.exit_code == 7
    assert json.loads(result.stdout)['exit_code'] == 7
    assert runner.invoke(main, ['--repo', str(repo)]).exit_code == 0


def test_missing_timeout(repo):
    result = asyncio.run(inspect(repo, ('/does-not-exist',), repo / 'x'))
    assert result.status == 'unavailable' and result.exit_code is None
    result = asyncio.run(inspect(repo, fake('import time; time.sleep(20)'), repo / 'x', .02))
    assert result.status == 'timeout' and result.exit_code is not None


def test_cancel_reaps(repo):
    pidfile = repo / 'pid'
    async def run():
        task = asyncio.create_task(inspect(repo, fake(f'import os,time; open({str(pidfile)!r},"w").write(str(os.getpid())); time.sleep(20)'), repo / 'x'))
        for _ in range(100):
            if pidfile.exists():
                break
            await asyncio.sleep(.01)
        assert pidfile.exists()
        task.cancel()
        with pytest.raises(asyncio.CancelledError):
            await task
        import os
        with pytest.raises(ProcessLookupError):
            os.kill(int(pidfile.read_text()), 0)
    asyncio.run(run())


def test_docs_mount_containment_lifespan(repo, tmp_path):
    outside = tmp_path / 'secret.md'
    outside.write_text('SECRET')
    (repo / 'docs/leak.md').symlink_to(outside)
    app = create_app(repo_root=repo, navigation=(("Workspace", "/external/"),))
    assert not app.state.started
    with TestClient(app) as client:
        assert app.state.started
        assert client.get('/').status_code == 200
        assert 'leak.md' not in client.get('/docs/').text
        assert client.get('/docs/leak.md').status_code == 404
        assert '&lt;script&gt;' in client.get('/docs/guide.md').text
        assert 'href="/docs/guide.md"' in client.get('/docs/README.md').text
        assert client.get('/diagnostics/').status_code == 200
    assert not app.state.started
    parent = Starlette(routes=[Mount('/custom', app=app)])
    with TestClient(parent, root_path='/deployment') as client:
        text = client.get('/deployment/custom/docs/README.md').text
        assert '/deployment/custom/static/style.css' in text
        assert '/deployment/custom/docs/guide.md' in text
        assert '/external/' in text
        guide = client.get('/deployment/custom/docs/guide.md')
        assert '/deployment/custom/docs/README.md' in guide.text
        assert client.get('/deployment/custom/static/style.css').status_code == 200
        assert '/deployment/custom/api/inspect' in client.get('/deployment/custom/diagnostics/').text


def test_instances_and_explicit_operation(repo, tmp_path):
    other = tmp_path / 'other'
    other.mkdir()
    (other / 'README.md').write_text('Other repository')
    command = fake('import os; print(os.getcwd())')
    for root in [repo, other]:
        app = create_app(repo_root=root, rust_command=command)
        with TestClient(app) as client:
            assert client.post('/api/inspect', json={'source': 'x.cart'}).json()['stdout'].strip() == str(root)


def test_real_binary():
    root = Path(__file__).resolve().parents[2]
    if not (root / 'target/debug/cartan-syntax').exists():
        pytest.skip('existing Rust binary unavailable; no implicit build')
    result = CliRunner().invoke(create_cli(repo_root=root), ['ir', str(root / 'tests/fixtures/diffusion.cart')])
    assert result.exit_code == 0
    assert result.stdout


def test_docs_do_not_spawn(repo, monkeypatch):
    def forbidden(*args, **kwargs):
        raise AssertionError("unexpected subprocess")
    monkeypatch.setattr(asyncio, 'create_subprocess_exec', forbidden)
    app = component()['create_app'](repo_root=repo)
    with TestClient(app) as client:
        for url in ['/', '/docs/', '/docs/README.md', '/docs/guide.md', '/diagnostics/', '/static/style.css']:
            assert client.get(url).status_code == 200
    assert CliRunner().invoke(create_cli(repo_root=repo)).exit_code == 0


def test_aggregate_enters_child_lifespan(repo):
    from contextlib import asynccontextmanager
    child = create_app(repo_root=repo)
    entries = []

    @asynccontextmanager
    async def lifespan(parent):
        async with child.router.lifespan_context(child):
            entries.append('start')
            yield
        entries.append('stop')

    parent = Starlette(routes=[Mount('/syntax', app=child)], lifespan=lifespan)
    with TestClient(parent) as client:
        assert child.state.started
        assert client.get('/syntax/').status_code == 200
    assert not child.state.started
    assert entries == ['start', 'stop']


def test_caller_relative_input(repo, tmp_path, monkeypatch):
    caller = repo / 'caller'
    caller.mkdir()
    monkeypatch.chdir(caller)
    cli = create_cli(repo_root=repo, rust_command=fake('import sys; print(sys.argv[-1])'))
    result = CliRunner().invoke(cli, ['ir', 'relative.cart'])
    assert result.exit_code == 0
    assert result.stdout.strip() == str(caller / 'relative.cart')


def test_readme_and_docs_root_symlinks(repo, tmp_path):
    secret = repo / 'secret.md'
    secret.write_text('SECRET')
    (repo / 'README.md').unlink()
    (repo / 'README.md').symlink_to(secret)
    with TestClient(create_app(repo_root=repo)) as client:
        assert client.get('/docs/README.md').status_code == 404
        assert client.get('/docs/%2e%2e/secret.md').status_code == 404


def test_standalone_help_without_repository(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    for args in ([], ['--help']):
        result = CliRunner().invoke(main, args)
        assert result.exit_code == 0
        assert all(word in result.stdout for word in ('ir', 'serve', '--repo', '--rust-command'))


@pytest.mark.parametrize('command', [(), ('',), ('python', None), 'python'])
def test_factory_rejects_invalid_command(repo, command):
    for factory in (create_cli, create_app):
        with pytest.raises(ValueError):
            factory(repo_root=repo, rust_command=command)


def test_configured_factories_and_explicit_override(repo):
    command = fake('print("configured")')
    (repo / 'pyproject.toml').write_text(
        '[tool.cartdev]\nrust-command = ' + json.dumps(list(command)) + '\n'
    )
    runner = CliRunner()
    configured = runner.invoke(create_cli(repo_root=repo), ['ir', 'x.cart'])
    assert configured.exit_code == 0 and configured.stdout == 'configured\n'
    overridden = runner.invoke(
        create_cli(repo_root=repo, rust_command=fake('print("explicit")')),
        ['ir', 'x.cart'],
    )
    assert overridden.exit_code == 0 and overridden.stdout == 'explicit\n'
    with TestClient(create_app(repo_root=repo)) as client:
        assert client.post('/api/inspect', json={'source': 'x.cart'}).json()['stdout'] == 'configured\n'


def test_named_mounts_with_distinct_roots_and_deployment_prefix(repo):
    other = repo / 'second'
    other.mkdir()
    (other / 'README.md').write_text('Second root')
    left = create_app(repo_root=repo)
    right = create_app(repo_root=other)
    parent = Starlette(routes=[
        Mount('/first', app=left, name='first'),
        Mount('/second', app=right, name='second'),
    ])
    with TestClient(parent, root_path='/deployed') as client:
        for prefix in ('first', 'second'):
            response = client.get(f'/deployed/{prefix}/docs/README.md')
            assert response.status_code == 200
            assert f'href="/deployed/{prefix}/static/style.css"' in response.text
            assert f'href="/deployed/{prefix}/docs/"' in response.text
        assert 'Second root' in client.get('/deployed/second/docs/README.md').text
        assert 'Second root' not in client.get('/deployed/first/docs/README.md').text


def test_nested_ir_and_machine_output(repo):
    command = fake('import json,os,sys; print(json.dumps([os.getcwd(),sys.argv[1:]]))')
    outer = click.Group('cartdev')
    outer.add_command(create_cli(repo_root=repo, rust_command=command))
    source = repo / 'spaces ; literal.cart'
    result = CliRunner().invoke(outer, ['syntax', 'ir', '--json', str(source)])
    assert result.exit_code == 0 and not result.stderr
    envelope = json.loads(result.stdout)
    assert set(envelope) == {'schema_version', 'status', 'exit_code', 'stdout', 'stderr'}
    assert envelope['schema_version'] == 1 and envelope['status'] == 'ok'
    assert json.loads(envelope['stdout']) == [str(repo), ['ast', str(source)]]


def test_document_front_matter_is_not_rendered(repo):
    (repo / 'docs/metadata.md').write_text('---\nid: private-metadata\n---\n# Visible title\n')
    with TestClient(create_app(repo_root=repo)) as client:
        response = client.get('/docs/metadata.md')
        assert 'private-metadata' not in response.text
        assert '<h1>Visible title</h1>' in response.text


def test_import_is_pure_in_fresh_interpreter():
    import subprocess
    code = '''
import asyncio, subprocess, socket
def forbidden(*args, **kwargs):
    raise AssertionError("unexpected external activity")
subprocess.Popen = forbidden
asyncio.create_subprocess_exec = forbidden
socket.create_connection = forbidden
import syntax_dev.application
assert syntax_dev.application.component()["api_version"] == 1
'''
    result = subprocess.run([sys.executable, '-c', code], capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
