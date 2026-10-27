"""Windows offline export. Preflight only unless --export is explicit."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

import contract as c

ROOT = Path(__file__).resolve().parents[2]


def git(root, *args):
    # No source-repository maintenance, hooks or optional index writes.
    return subprocess.check_output(['git', '--no-replace-objects', '--no-optional-locks',
        '-c', 'maintenance.auto=false', '-c', 'gc.auto=0', '-c', 'core.hooksPath=' + os.devnull,
        '-c', 'core.fsmonitor=false', '-c', 'filter.lfs.process=', '-c', 'filter.lfs.clean=',
        '-c', 'filter.lfs.smudge=', '-c', 'filter.lfs.required=false', *args],
        cwd=root, timeout=180, env=c.git_environment(os.environ, os.devnull))


def inventory(root, source):
    c.require(c.oid(source) and git(root, 'rev-parse', 'HEAD').decode().strip() == source, 'exact explicit HEAD')
    c.require(not git(root, 'status', '--porcelain', '--untracked-files=all'), 'commit source before export')
    files = c.tree(git(root, 'ls-tree', '-r', '-l', '-z', source))
    names = c.refs(git(root, 'for-each-ref', '--format=%(objectname) %(refname)'))
    c.require(c.sha((root / c.QUAL).read_bytes()) == c.QUAL_SHA, 'qualification drift')
    return files, names


def run(source, export=False):
    c.require(os.name == 'nt', 'Windows exporter only')
    files, names = inventory(ROOT, source)
    total = sum(row['bytes'] for row in files.values())
    # The transfer estimate does not authorize filling the physical volume.
    c.space(shutil.disk_usage(ROOT).free, total, 512 * 1024**2)
    summary = dict(source=source, files=len(files), blob_bytes=total,
        refs=len(names), export=False, checkout_created=False, package_pass=False, release_admitted=False)
    if not export:
        return summary
    out = ROOT / 'target/native-frontier-20261027' / source
    out.mkdir(parents=True, exist_ok=False)
    def write(name, value):
        with (out / name).open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write('\n')
    write('intent.json', dict(source=source, tree=files, refs=names, retry=False, server=False, services=False,
        clocks=False, package_dispatched=False, release_admitted=False))
    try:
        bundle = out / 'history.bundle'
        git(ROOT, 'bundle', 'create', str(bundle), '--all')
        c.require(inventory(ROOT, source) == (files, names), 'source/index/refs changed during export')
        raw = bundle.read_bytes()
        c.space(shutil.disk_usage(ROOT).free, total, len(raw))
        value = c.request(dict(schema='hc074-native-frontier-transfer-v1', source=source,
            tree=files, refs=names, bundle_sha256=c.sha(raw), bundle_bytes=len(raw), qualifier_sha256=c.QUAL_SHA))
        write('request.json', value)
        summary.update(export=True, bundle_bytes=len(raw), bundle_sha256=c.sha(raw),
            request_sha256=c.sha((out / 'request.json').read_bytes()))
        write('result.json', summary)
    except BaseException as error:
        write('refusal.json', dict(type=type(error).__name__, reason=str(error), retry=False, release_admitted=False))
        raise
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True)
    parser.add_argument('--export', action='store_true')
    args = parser.parse_args()
    print(json.dumps(run(args.source, args.export), sort_keys=True))
