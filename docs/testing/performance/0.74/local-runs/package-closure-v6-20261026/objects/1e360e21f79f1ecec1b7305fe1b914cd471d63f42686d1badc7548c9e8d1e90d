"""Fresh synthetic roots and independent Git references; no Rust stream-derived pins."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

import reader


def profiles(fault_controls, replaced):
    if not isinstance(fault_controls, bool) or not isinstance(replaced, bool):
        raise ValueError('explicit fixture modes required')
    if fault_controls:
        return [('default' if replaced else 'disabled', False)]
    return [('disabled', False), ('default', False), ('default', True)]


def prepare(root, *, fault_controls=False):
    profiles(fault_controls, False)  # Refuse invalid mode before fixture effects.
    root = Path(root).resolve(strict=True)
    if not root.is_dir() or any(root.iterdir()):
        raise ValueError('existing empty fixture root required')
    # This owned diagnostic process, not the caller or host, has isolated Git config.
    environment = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
    environment.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull,
                       GIT_ATTR_NOSYSTEM='1', GIT_NO_LAZY_FETCH='1',
                       GIT_AUTHOR_DATE='2000-01-01T00:00:00+00:00',
                       GIT_COMMITTER_DATE='2000-01-01T00:00:00+00:00')
    os.environ.clear()
    os.environ.update(environment)

    def git(repo, policy, *args, data=None, success=True):
        flags = ['--no-replace-objects'] if policy == 'disabled' else []
        result = subprocess.run(['git', *flags, *args], cwd=repo, input=data,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10)
        if success and result.returncode != 0:
            raise ValueError(f'fixture Git failed: {args!r}: {result.stderr!r}')
        return result

    def build(name, replaced):
        repo = root / name
        repo.mkdir()
        git(repo, 'disabled', 'init', '--object-format=sha1', '--initial-branch=fixture')
        for key, value in [('user.name', 'Synthetic control'), ('user.email', 'fixture@example.invalid'),
                           ('core.autocrlf', 'false'), ('commit.gpgsign', 'false')]:
            git(repo, 'disabled', 'config', key, value)
        empty_hooks = repo / 'empty-hooks'
        empty_hooks.mkdir()
        git(repo, 'disabled', 'config', 'core.hooksPath', str(empty_hooks))
        files = {'empty': b'', 'binary': bytes(range(256)), 'space-λ': b'raw\r\nbytes\0',
                 'folder/large': b'a\0\xff\n' * 65536}
        for path, body in files.items():
            target = repo / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(body)
        git(repo, 'disabled', 'add', '--all')
        link = git(repo, 'disabled', 'hash-object', '-w', '--stdin', data=b'folder/large').stdout.decode().strip()
        git(repo, 'disabled', 'update-index', '--add', '--cacheinfo', f'120000,{link},link')
        git(repo, 'disabled', 'commit', '--no-verify', '-m', 'synthetic one')
        first = git(repo, 'disabled', 'rev-parse', 'HEAD').stdout.decode().strip()
        old_blob = git(repo, 'disabled', 'rev-parse', first + ':binary').stdout.decode().strip()
        (repo / 'binary').write_bytes(b'second commit bytes')
        git(repo, 'disabled', 'add', 'binary')
        git(repo, 'disabled', 'commit', '--no-verify', '-m', 'synthetic two')
        second = git(repo, 'disabled', 'rev-parse', 'HEAD').stdout.decode().strip()
        if replaced:
            new_blob = git(repo, 'disabled', 'hash-object', '-w', '--stdin', data=b'replacement bytes').stdout.decode().strip()
            git(repo, 'disabled', 'replace', old_blob, new_blob)
        # Deliberately different current bytes. Native transport must not read them.
        (repo / 'binary').write_bytes(b'uncommitted current file must stay separate')
        return repo, first, second

    def output(result):
        return dict(stdout_hex=result.stdout.hex(), stderr_hex=result.stderr.hex(), code=result.returncode)

    cases = []
    for name, replaced in [('plain', False), ('replaced', True)]:
        repo, first, second = build(name, replaced)
        for policy, short in profiles(fault_controls, replaced):
            revision = first[:8] if short else first
            specs = [revision + ':' + path for path in
                     ('binary', 'empty', 'space-λ', 'folder/large', 'link', 'missing', 'folder', 'binary')]
            specs.append(second + ':binary')
            pins, outputs = [], []
            for spec in specs:
                outputs.append(output(git(repo, policy, 'show', spec, success=False)))
                kind = git(repo, policy, 'cat-file', '-t', spec, success=False)
                if kind.returncode != 0:
                    pins.append(dict(kind='missing', spec=spec, short=short and spec.startswith(revision + ':')))
                    continue
                kind = kind.stdout.decode().strip()
                oid = git(repo, policy, 'rev-parse', '--verify', spec).stdout.decode().strip()
                body = git(repo, policy, 'cat-file', kind, spec).stdout
                pins.append(dict(kind=kind, oid=oid, bytes=len(body), sha256=hashlib.sha256(body).hexdigest()))
            python_prototype = None
            if policy == 'disabled':
                values = reader.read_batch(repo, [reader.Request(*spec.split(':', 1)) for spec in specs])
                python_prototype = [dict(stdout_hex=v.stdout.hex(), stderr_hex=v.stderr.hex(), code=v.returncode) for v in values]
                if python_prototype != outputs:
                    raise ValueError('Python prototype differs from independent show reference')
            cases.append(dict(root=name, policy=policy, short=short, replaced=replaced,
                              specs=specs, pins=pins, expected=outputs, python_prototype=python_prototype))
    git_path = Path(shutil.which('git')).resolve(strict=True)
    return dict(schema=('hc074-blob-fault-synthetic-oracle-v1' if fault_controls
                        else 'hc074-blob-native-synthetic-oracle-v1'), cases=cases,
                git_version=git(root / 'plain', 'disabled', '--version').stdout.decode().strip(),
                git_path=str(git_path), git_executable_sha256=hashlib.sha256(git_path.read_bytes()).hexdigest(),
                source_authority=False, measurement=False, server=False)


if __name__ == '__main__':
    if len(sys.argv) not in (2, 3) or (len(sys.argv) == 3 and sys.argv[2] != '--fault-controls'):
        raise SystemExit('one existing empty fixture root required')
    print(json.dumps(prepare(sys.argv[1], fault_controls=len(sys.argv) == 3), sort_keys=True))
