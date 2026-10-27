"""One independent native checkout. Run only under the pinned CI heartbeat.

No tests, service, network, installation, source fallback, reuse or cleanup CLI.
Incomplete directories and their first receipts are deliberately retained.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess

import contract as c

BASE = Path('/home/artur/.cache/hydracache-074-frontier-native')


def git(root, *args):
    return subprocess.check_output(['/usr/bin/git', '--no-replace-objects', '--no-optional-locks',
        '-c', 'core.hooksPath=/dev/null', '-c', 'maintenance.auto=false', '-c', 'gc.auto=0',
        *args], cwd=root, env=c.git_environment(os.environ))


def parents():
    c.require(os.name == 'posix' and os.geteuid() != 0, 'nonroot Linux preparation only')
    for path in (Path('/home/artur'), Path('/home/artur/.cache')):
        metadata = path.lstat()
        c.require(stat.S_ISDIR(metadata.st_mode) and metadata.st_uid == os.geteuid()
                  and not metadata.st_mode & 0o022 and path.resolve() == path, 'native ancestor custody')
    c.require(subprocess.check_output(['/usr/bin/findmnt', '-n', '-o', 'FSTYPE', '-T', str(BASE.parent)]).strip() == b'ext4',
              'native ext4 filesystem required')


def validate_files(repo, expected, check_space):
    actual = c.tree(git(repo, 'ls-tree', '-r', '-l', '-z', 'HEAD'))
    c.require(actual == expected, 'complete native tree differs')
    for at, (name, row) in enumerate(expected.items()):
        if at % 256 == 0:
            check_space()
        path = repo / name
        metadata = path.lstat()
        c.require(stat.S_ISREG(metadata.st_mode) and path.resolve() == repo.resolve() / name, 'nonregular native file')
        raw = path.read_bytes()
        c.require(len(raw) == row['bytes'] and c.blob_digest(raw) == row['blob'], 'native file bytes differ: ' + name)
        c.require(bool(metadata.st_mode & 0o111) == (row['mode'] == '100755'), 'native executable mode differs')
    c.require(c.sha((repo / c.QUAL).read_bytes()) == c.QUAL_SHA, 'native qualifier differs')
    c.require(not git(repo, 'status', '--porcelain', '--untracked-files=all'), 'native status not clean')


def run(request_path, request_sha):
    parents()
    raw = request_path.read_bytes()
    c.require(c.sha(raw) == request_sha, 'request binding differs')
    value = c.request(c.decode(raw))
    total = sum(row['bytes'] for row in value['tree'].values())
    c.space(shutil.disk_usage('/mnt/c').free, total, value['bundle_bytes'])
    bundle = request_path.parent / 'history.bundle'
    raw_bundle = bundle.read_bytes()
    c.require(len(raw_bundle) == value['bundle_bytes'] and c.sha(raw_bundle) == value['bundle_sha256'], 'bundle bytes differ')
    if BASE.exists() or BASE.is_symlink():
        metadata = BASE.lstat()
        c.require(stat.S_ISDIR(metadata.st_mode) and metadata.st_uid == os.geteuid()
                  and not metadata.st_mode & 0o077 and BASE.resolve() == BASE, 'native base custody')
    else:
        BASE.mkdir(mode=0o700)
    out = BASE / value['source']
    out.mkdir(mode=0o700, exist_ok=False)
    def write(name, body):
        with (out / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(body, stream, sort_keys=True, indent=2)
            stream.write('\n')
    write('intent.json', dict(request_sha256=request_sha, source=value['source'], retry=False,
        tests_dispatched=False, services=False, clocks=False, server=False, release_admitted=False))
    def check_space():
        c.require(shutil.disk_usage('/mnt/c').free >= c.FLOOR, 'physical backing floor during materialization')
        c.require(shutil.disk_usage(out).free >= c.FLOOR, 'native filesystem floor')
    try:
        local_bundle = out / 'history.bundle'
        with local_bundle.open('xb') as stream:
            stream.write(raw_bundle)
        repo = out / 'repo'
        repo.mkdir(mode=0o700)
        c.require('refs/heads/hc074-native-transfer-unborn' not in value['refs'], 'unborn branch collision')
        git(repo, 'init', '--template=', '--initial-branch=hc074-native-transfer-unborn', '.')
        git(repo, 'config', 'core.autocrlf', 'false')
        git(repo, 'config', 'core.filemode', 'true')
        check_space()
        git(repo, 'bundle', 'verify', str(local_bundle))
        git(repo, '-c', 'protocol.file.allow=always', 'fetch', '--no-tags', '--no-write-fetch-head',
            str(local_bundle), '+refs/*:refs/*')
        c.require(c.refs(git(repo, 'for-each-ref', '--format=%(objectname) %(refname)')) == value['refs'], 'native refs differ')
        c.require(not (repo / '.git/objects/info/alternates').exists()
                  and not (repo / '.git/commondir').exists(), 'borrowed object database')
        check_space()
        git(repo, 'checkout', '--detach', value['source'])
        c.require(git(repo, 'rev-parse', 'HEAD').decode().strip() == value['source'], 'native HEAD differs')
        validate_files(repo, value['tree'], check_space)
        git(repo, 'fsck', '--connectivity-only', '--no-reflogs')
        check_space()
        result = dict(source=value['source'], request_sha256=request_sha, repo=str(repo), files=len(value['tree']),
            blob_bytes=total, refs=len(value['refs']), all_file_bytes_and_modes_exact=True,
            independent_object_database=True, source_tree_unchanged=True, tests_dispatched=False,
            linux_frontier_complete=False, package_pass=False, release_admitted=False)
        write('result.json', result)
        print(json.dumps(result, sort_keys=True))
        return result
    except BaseException as error:
        write('refusal.json', dict(type=type(error).__name__, reason=str(error), retry=False,
            tests_dispatched=False, release_admitted=False))
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--request', type=Path, required=True)
    parser.add_argument('--request-sha256', required=True)
    args = parser.parse_args()
    run(args.request, args.request_sha256)
