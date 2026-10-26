"""Bounded source-only Rust AST inventory; no input code execution or live tests."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

import package_replay_audit


def module_edges(root, document):
    additions, unresolved = {}, []
    known = {v['source'] for v in document['sources']}
    for row in document['sources']:
        source = root / row['source']
        for site in row['sites']:
            if site['kind'] != 'module':
                continue
            parts = site['scope'].split('::')
            if site['detail'].startswith('literal:'):
                candidates = [source.parent.joinpath(*parts[:-1], site['detail'][8:])]
            else:
                assert site['detail'] == 'default-module-path'
                base = source.parent if source.name in ('lib.rs', 'main.rs', 'mod.rs') else source.with_suffix('')
                candidates = [base.joinpath(*parts).with_suffix('.rs'), base.joinpath(*parts, 'mod.rs')]
            existing = [p.resolve() for p in candidates if p.is_file()]
            if len(existing) != 1:
                unresolved.append(dict(source=row['source'], scope=site['scope'], candidates=[str(v) for v in candidates]))
                continue
            path, = existing
            assert path.is_relative_to(root)
            name = path.relative_to(root).as_posix()
            if name not in known:
                additions[name] = hashlib.sha256(path.read_bytes()).hexdigest()
    return additions, unresolved


def capture(root, build, out):
    out.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (out / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write('\n')
    write('intent.json', dict(build=str(build), timeout_seconds=120, source_only=True,
          package_replay=False, services=False, server=False, clocks_changed=False))
    try:
        audit = package_replay_audit.audit(root)
        request = dict(sources=audit['sources'])
        receipt = json.loads((build / 'result.json').read_text())
        assert receipt['exit'] == 0 and receipt['source_unchanged'] and receipt['head_unchanged']
        build_sources = json.loads((build / 'intent.json').read_text())['source']
        assert all(build_sources.get(name) == digest for name, digest in request['sources'].items())
        messages = [json.loads(line) for line in (build / 'stdout.log').read_text().splitlines() if line.startswith('{')]
        artifact, = [v for v in messages if v.get('reason') == 'compiler-artifact'
                     and v.get('target', {}).get('name') == 'census_cost_074']
        binary = Path(artifact['executable']).resolve()
        assert binary.parent == (root / 'target/debug/deps').resolve() and artifact['profile']['test']
        for round_number in range(8):
            suffix = '' if round_number == 0 else '-' + str(round_number)
            write('input' + suffix + '.json', request)
            assert all(build_sources.get(name) == digest for name, digest in request['sources'].items())
            with (out / ('stdout' + suffix + '.log')).open('xb') as stdout, (out / ('stderr' + suffix + '.log')).open('xb') as stderr:
                child = subprocess.run([str(binary), '--exact', 'package_syntax_child', '--nocapture', '--test-threads=1'],
                    cwd=root, env=dict(os.environ, HC074_PACKAGE_SYNTAX_INPUT=str((out / ('input' + suffix + '.json')).resolve())),
                    stdout=stdout, stderr=stderr, timeout=120, check=False)
            assert child.returncode == 0
            prefix = 'test package_syntax_child ... '
            lines = [(v[len(prefix):] if v.startswith(prefix + 'HC074_SYNTAX ') else v)
                     for v in (out / ('stdout' + suffix + '.log')).read_text().splitlines()]
            result, = [json.loads(v[len('HC074_SYNTAX '):]) for v in lines if v.startswith('HC074_SYNTAX ')]
            additions, unresolved = module_edges(root, result)
            write('edges' + suffix + '.json', dict(additions=additions, unresolved=unresolved))
            assert not unresolved, 'unresolved actual module edge'
            if not additions:
                break
            request = dict(sources=dict(sorted({**request['sources'], **additions}.items())))
        else:
            raise ValueError('module closure bound')
        assert not result['executed_input_code'] and not result['compiler_expansion']
        assert {v['source']: v['sha256'] for v in result['sources']} == request['sources']
        for name, digest in request['sources'].items():
            assert hashlib.sha256((root / name).read_bytes()).hexdigest() == digest
        write('result.json', result)
        print(json.dumps(dict(source_files=len(result['sources']), syntax_sites=sum(len(v['sites']) for v in result['sources']),
              binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), source_only=True)))
    except BaseException as error:
        write('refusal.json', dict(type=type(error).__name__, reason=str(error)))
        raise


if __name__ == '__main__':
    capture(Path(__file__).resolve().parents[2], Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
