import json
from pathlib import Path
import re
import subprocess
import sys

from click.testing import CliRunner
import pytest

from syntax_dev.application import create_cli
from syntax_dev.forward import present
from syntax_dev.metadata import MetadataError, discover, outline

ROOT = Path(__file__).resolve().parents[2]
BINARY = ROOT / 'target/debug/cartan-syntax'


def argument(id, **fields):
    value = {
        'id': id, 'description': None, 'long': None, 'short': None, 'aliases': [],
        'short_aliases': [], 'position': None, 'value_names': [],
        'arity': {'min': 0, 'max': 0}, 'action': 'set_true', 'repeatable': False,
        'required': False, 'global': False, 'default_values': [], 'choices': [],
    }
    value.update(fields)
    return value


def node(name, description, arguments=(), commands=()):
    return {'name': name, 'description': description, 'aliases': [],
            'arguments': list(arguments), 'commands': list(commands)}


def tree(*commands):
    return {
        'schema_version': 1, 'program': 'cartan-syntax', 'version': '0.1.0',
        'command': node('cartan-syntax', 'Inspect Cartan source syntax', [
            argument('color', long='color', value_names=['WHEN'], arity={'min': 1, 'max': 1},
                     action='set', default_values=['auto'],
                     choices=['auto', 'always', 'never'], description='Control terminal colors')
            | {'global': True},
            argument('help-short', short='h', action='help_short', description='Print concise help'),
        ], commands),
    }


IR = node('ir', 'Print the normalized source AST and byte spans', [
    argument('source', position=1, value_names=['SOURCE'], arity={'min': 1, 'max': 1},
             action='set', required=True, description='Cartan source file'),
])
LINT = node('lint', 'Check source style', [
    argument('strict', long='strict', description='Fail on warnings'),
], [node('deep', 'Check nested forms', [
    argument('limit', long='limit', value_names=['N'], arity={'min': 1, 'max': 1},
             action='set', default_values=['3'], description='Nesting limit'),
])])


def fake(tmp_path, metadata, *, status=0, raw=None):
    """A Rust CLI stand-in: metadata for --describe-cli, otherwise its argv as JSON."""
    script = tmp_path / 'fake_rust.py'
    payload = raw if raw is not None else json.dumps(metadata)
    script.write_text(
        'import json, sys\n'
        f'if sys.argv[1:] == ["--describe-cli"]:\n'
        f'    sys.stdout.write({payload!r}); sys.exit({status})\n'
        'print(json.dumps(sys.argv[1:]))\n'
    )
    return (sys.executable, str(script))


def test_added_and_removed_metadata_commands_appear_without_registration(tmp_path):
    runner = CliRunner()
    with_lint = create_cli(repo_root=tmp_path, rust_command=fake(tmp_path, tree(IR, LINT)))
    concise = runner.invoke(with_lint, ['-h'])
    assert concise.exit_code == 0
    assert 'lint   Check source style' in concise.stdout
    assert 'Command outline:' not in concise.stdout
    full = runner.invoke(with_lint, ['--help'])
    assert full.exit_code == 0
    assert '  syntax lint  Check source style\n      --strict  Fail on warnings' in full.stdout
    assert '  syntax lint deep  Check nested forms\n      --limit <N>  Nesting limit [default: 3]' in full.stdout
    # A generated command's arguments reach Rust as written, never reparsed.
    args = ['lint', 'deep', '--limit', 'not-a-number', '--', '-x path.cart']
    result = runner.invoke(with_lint, args)
    assert result.exit_code == 0
    assert json.loads(result.stdout) == args
    without_lint = create_cli(repo_root=tmp_path, rust_command=fake(tmp_path, tree(IR)))
    assert 'lint' not in runner.invoke(without_lint, ['--help']).stdout


def test_metadata_failures_are_typed_and_keep_python_commands(tmp_path):
    cases = [
        ({'raw': 'not json'}, 'json'),
        ({'metadata': {**tree(IR), 'schema_version': 2}}, 'schema'),
        ({'metadata': tree(node('ir', 'x', [{'id': 'broken'}]))}, 'schema'),
        ({'metadata': tree(IR), 'status': 3}, 'status'),
        ({'metadata': tree(node('serve', 'Shadow a Python command'))}, 'collision'),
    ]
    for options, kind in cases:
        command = fake(tmp_path, options.get('metadata'), status=options.get('status', 0),
                       raw=options.get('raw'))
        if kind != 'collision':
            with pytest.raises(MetadataError) as raised:
                discover(command)
            assert raised.value.kind == kind
        result = CliRunner().invoke(create_cli(repo_root=tmp_path, rust_command=command), ['-h'])
        assert result.exit_code == 0
        assert 'serve' in result.stdout
        assert 'Rust commands unavailable' in result.stdout
    with pytest.raises(MetadataError) as raised:
        discover(fake(tmp_path, tree(node('ir', 'x', [{'id': 'broken'}]))))
    assert raised.value.detail == 'command.commands[0].arguments[0].description'
    assert 'schema version 1' in present(raised.value)


def test_discovery_runs_once_per_group(tmp_path, monkeypatch):
    calls = []
    real = subprocess.run
    def counting(command, *args, **kwargs):
        calls.append(command)
        return real(command, *args, **kwargs)
    monkeypatch.setattr(subprocess, 'run', counting)
    cli = create_cli(repo_root=tmp_path, rust_command=fake(tmp_path, tree(IR, LINT)))
    assert CliRunner().invoke(cli, ['--help']).exit_code == 0
    assert sum(command[-1] == '--describe-cli' for command in calls) == 1


@pytest.mark.skipif(not BINARY.exists(), reason='explicit Rust build required; no implicit build')
def test_actual_binary_outline_and_commands_match_rust_help():
    metadata = json.loads(subprocess.run([str(BINARY), '--describe-cli'], capture_output=True,
                                         check=True).stdout)
    rust_help = subprocess.run([str(BINARY), '--help'], capture_output=True, check=True,
                               env={'NO_COLOR': '1'}).stdout.decode()
    rust_outline = rust_help.split('Command outline:\n', 1)[1].rstrip('\n').splitlines()
    assert outline(metadata['command'], ('cartan-syntax',)) == rust_outline
    cli = create_cli(repo_root=ROOT)
    concise = CliRunner().invoke(cli, ['-h'])
    full = CliRunner().invoke(cli, ['--help'])
    assert concise.exit_code == full.exit_code == 0
    for command in metadata['command']['commands']:
        assert re.search(rf"^  {command['name']} +{re.escape(command['description'])}$",
                         concise.stdout, re.M)
    python_outline = full.stdout.split('Command outline:\n', 1)[1].splitlines()
    derived = [line.replace('  syntax', '  cartan-syntax', 1) for line in python_outline]
    assert derived[:len(rust_outline)] == rust_outline
    assert not any(command['name'] == 'help' for command in metadata['command']['commands'])


def test_outline_rules_match_the_rust_unit_test_lines():
    # Mirrors outline_orders_nested_paths_positionals_repeats_defaults_and_choices
    # in cli/src/command.rs: same tree, same expected lines.
    value = dict(arity={'min': 1, 'max': 1}, action='set')
    tool = node('tool', 'A tool', [
        argument('param', long='param', value_names=['NAME=VALUE'], repeatable=True,
                 description='Static parameter', **{**value, 'action': 'append'}),
    ], [
        node('emit', 'Emit code', [
            argument('target', position=1, value_names=['TARGET'], required=True,
                     choices=['metal', 'cuda', 'wasm'], description='Target', **value),
            argument('source', position=2, value_names=['SOURCE'],
                     description='Source file', **value),
        ]),
        node('wasm', 'Wasm tools', [], [node('validate', None, [], [
            node('diffusion', 'Validate diffusion', [
                argument('offset', long='offset', value_names=['X'], default_values=['0.5'],
                         description='Offset', **value),
            ]),
        ])]),
    ])
    assert outline(tool, ('tool',)) == [
        '  tool  A tool',
        '      --param <NAME=VALUE>...  Static parameter',
        '  tool emit  Emit code',
        '      <TARGET>  Target [choices: metal, cuda, wasm]',
        '      [SOURCE]  Source file',
        '  tool wasm  Wasm tools',
        '  tool wasm validate',
        '  tool wasm validate diffusion  Validate diffusion',
        '      --offset <X>  Offset [default: 0.5]',
    ]


def test_hidden_python_commands_dispatch_without_listing(tmp_path):
    import click
    cli = create_cli(repo_root=tmp_path, rust_command=fake(tmp_path, tree(IR)))

    @cli.command(hidden=True)
    def worker():
        click.echo('worker ran')

    runner = CliRunner()
    assert runner.invoke(cli, ['worker']).stdout == 'worker ran\n'
    for flag in ('-h', '--help'):
        assert 'worker' not in runner.invoke(cli, [flag]).stdout


def test_malformed_shapes_are_schema_errors_and_keep_python_help(tmp_path):
    def mutate(change):
        value = json.loads(json.dumps(tree(IR)))
        change(value)
        return value
    source = lambda value: value['command']['commands'][0]['arguments'][0]
    cases = {
        'command.commands[0].arguments[0].action': mutate(lambda v: source(v).update(action=[])),
        'command.commands[0].arguments[0].arity.max': mutate(lambda v: source(v)['arity'].pop('max')),
        'command.commands[0].arguments[0].arity': mutate(lambda v: source(v).update(arity=[])),
        'schema_version': mutate(lambda v: v.update(schema_version=True)),
        'command.commands[0].name': mutate(lambda v: v['command']['commands'][0].update(name=None)),
    }
    for path, metadata in cases.items():
        command = fake(tmp_path, metadata)
        with pytest.raises(MetadataError) as raised:
            discover(command)
        assert (raised.value.kind, raised.value.detail) == ('schema', path)
        for flag in ('-h', '--help'):
            result = CliRunner().invoke(create_cli(repo_root=tmp_path, rust_command=command), [flag])
            assert result.exit_code == 0
            assert 'serve' in result.stdout
            assert f'incompatible at {path}' in result.stdout


def test_nested_metadata_commands_are_traversable_click_groups(tmp_path):
    import click
    cli = create_cli(repo_root=tmp_path, rust_command=fake(tmp_path, tree(IR, LINT)))
    ctx = click.Context(cli, info_name='syntax')
    lint = cli.get_command(ctx, 'lint')
    assert isinstance(lint, click.Group)
    assert lint.help == 'Check source style'
    deep = lint.get_command(ctx, 'deep')
    assert deep.help == 'Check nested forms' and not isinstance(deep, click.Group)
    assert not isinstance(cli.get_command(ctx, 'ir'), click.Group)
    # Traversal never changes invocation: the root still forwards raw argv.
    args = ['lint', 'deep', '--unknown']
    assert json.loads(CliRunner().invoke(cli, args).stdout) == args


def test_python_option_without_default_renders_no_default(tmp_path):
    import click
    cli = create_cli(repo_root=tmp_path, rust_command=fake(tmp_path, tree(IR)))

    @cli.command()
    @click.option("--name", help="Optional name.")
    def label(name):
        pass

    full = CliRunner().invoke(cli, ['--help']).stdout
    line = next(line for line in full.splitlines() if '--name' in line)
    assert line == '      --name <NAME>  Optional name.'
    assert 'UNSET' not in full
