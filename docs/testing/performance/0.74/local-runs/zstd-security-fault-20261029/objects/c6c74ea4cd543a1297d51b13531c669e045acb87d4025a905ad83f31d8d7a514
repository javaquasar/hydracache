"""Offline current-namespace join; never equates a later PID with a trace PID."""
import argparse
import json
import re

import run as base

OUT = base.ROOT / 'target/wsl-clock-writer-identity-20261023'
ARCHIVE = base.ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-writer-identity-20261023'
CAPTURE_SHA = '6b54d38f0ce7b4ceacdeb0ce879a7aee6570e53501cfc12206b39c78eaef051d'
BOOT = 'cf222c07-5476-4f70-9a32-455e99204d0a'
FALSE_FLAGS = ('trace_capture', 'tracing_state_changed', 'clock_change', 'service_change',
               'original_owner_run', 'server', 'product', 'admission')


def fields(raw):
    result = {}
    for name, value, rc in re.findall(r'^FIELD (\S+)\n(.*?)\nRESULT (\d+)\n', raw, re.M | re.S):
        if name in result or int(rc):
            raise ValueError('Duplicate or failed metadata field: ' + name)
        result[name] = value.strip()
    return result


def stat(raw):
    pid, tail = raw.split(' (', 1)
    comm, tail = tail.rsplit(') ', 1)
    return int(pid), comm, int(tail.split()[19])


def context(raw):
    # Decode Windows pipe/fixture line endings without rewriting archived bytes.
    raw = raw.replace('\r\n', '\n')
    blocks = re.split(r'^PROCESS (\d+) ([^\n]+)\n', raw, flags=re.M)
    header = fields(blocks[0])
    footer = fields(blocks[-1])
    if header['boot'] != BOOT or footer['boot-after'] != BOOT:
        raise ValueError('Different boot context')
    result = []
    for i in range(1, len(blocks), 3):
        pid, comm, body = int(blocks[i]), blocks[i + 1], blocks[i + 2]
        f = fields(body)
        before, after = stat(f['stat-before']), stat(f['stat-after'])
        status = dict(line.split(':', 1) for line in f['status'].splitlines())
        chain = [int(x) for x in status['NSpid'].split()]
        digest, path = f['executable-digest'].split()
        if (before != after or before[:2] != (pid, comm) or int(status['Pid']) != pid
                or status['Name'].strip() != comm or not chain or chain[0] != pid
                or not re.fullmatch('[0-9a-f]{64}', digest) or path != f'/proc/{pid}/exe'):
            raise ValueError('Inconsistent current process metadata')
        result.append({'pid': pid, 'comm': comm, 'start_ticks': before[2], 'nspid': chain,
                       'uid': [int(x) for x in status['Uid'].split()],
                       'exe': f['exe'], 'sha256': digest, 'cgroup': f['cgroup'],
                       'pidns': f['ns/pid'], 'timens': f['ns/time'], 'mntns': f['ns/mnt']})
    return {'observer_pidns': header['observer-pidns'],
            'observer_timens': header['observer-timens'], 'processes': result}


def summarize(system, ubuntu, docker):
    contexts = {name: context(raw) for name, raw in
                [('system', system), ('ubuntu', ubuntu), ('docker', docker)]}
    selected = []
    for name in ('system', 'ubuntu'):
        candidates = [p for p in contexts[name]['processes'] if p['comm'] == 'systemd-timesyn']
        if len(candidates) != 1:
            raise ValueError('Expected exactly one current timesyn candidate per view')
        selected.append(candidates[0])
    a, b = selected
    keys = ('start_ticks', 'uid', 'exe', 'sha256', 'cgroup', 'pidns', 'timens', 'mntns')
    if (any(a[k] != b[k] for k in keys) or len(a['nspid']) != len(b['nspid']) + 1
            or a['nspid'][1:] != b['nspid'] or a['timens'] != contexts['ubuntu']['observer_timens']):
        raise ValueError('Cannot join current cross-namespace process views')
    return {'method': 'hc074-post-capture-current-namespace-join-v1', 'boot': BOOT,
            'source_capture_sha256': CAPTURE_SHA, 'contexts': contexts,
            'current_timesyn_views_join': True, 'current_timesyn': selected,
            'trace_pid_to_executable_authenticated': False, 'historical_tick_setter': None,
            'root_debug_shell': 'refused: Windows Administrator required',
            'original_windows': [300, 60, 10], **dict.fromkeys(FALSE_FLAGS, False)}


def analyze():
    return summarize(*[(OUT / (name + '-root-processes.stdout.log')).read_text(encoding='utf-8')
                       for name in ('system', 'ubuntu', 'docker')])


def retain():
    base.sources()  # Frozen runtime, qualification and older first outcomes.
    capture = base.ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-setter-20261023/index.json'
    if base.digest(capture.read_bytes()) != CAPTURE_SHA:
        raise ValueError('Changed source capture index')
    summary = analyze()
    base.put(OUT / 'summary.json', summary)
    ARCHIVE.mkdir(exist_ok=False)
    objects = ARCHIVE / 'objects'
    objects.mkdir()
    files = {}
    inputs = [('observation/' + p.name, p.read_bytes()) for p in sorted(OUT.iterdir()) if p.is_file()]
    inputs += [('source/' + p.name, p.read_bytes()) for p in
               (base.HERE / 'identity.py', base.HERE / 'test_identity.py')]
    for name, raw in inputs:
        sha = base.digest(raw)
        if not (objects / sha).exists():
            with (objects / sha).open('xb') as stream:
                stream.write(raw)
        files[name] = {'sha256': sha, 'bytes': len(raw)}
    base.put(ARCHIVE / 'index.json', {'schema': 'hc074-readonly-writer-identity-v1',
             'files': files, 'source_capture_sha256': CAPTURE_SHA,
             'original_windows': [300, 60, 10], **dict.fromkeys(FALSE_FLAGS, False)})
    verify()


def verify():
    capture = base.ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-setter-20261023/index.json'
    if base.digest(capture.read_bytes()) != CAPTURE_SHA:
        raise ValueError('Changed source capture index')
    index = json.loads((ARCHIVE / 'index.json').read_bytes())
    if (index['schema'] != 'hc074-readonly-writer-identity-v1'
            or index['source_capture_sha256'] != CAPTURE_SHA
            or index['original_windows'] != [300, 60, 10]
            or any(index[k] is not False for k in FALSE_FLAGS)):
        raise ValueError('Unexpected identity evidence scope')
    files = {}
    for name, item in index['files'].items():
        sha = item['sha256']
        if not re.fullmatch('[0-9a-f]{64}', sha):
            raise ValueError('Unsafe object digest')
        raw = (ARCHIVE / 'objects' / sha).read_bytes()
        if base.digest(raw) != sha or len(raw) != item['bytes']:
            raise ValueError('Changed identity evidence')
        files[name] = raw
    result = summarize(*[files['observation/' + name + '-root-processes.stdout.log'].decode()
                         for name in ('system', 'ubuntu', 'docker')])
    if json.loads(files['observation/summary.json']) != result:
        raise ValueError('Saved identity summary does not reproduce')
    print(json.dumps({'files': len(files), 'index_sha256': base.digest((ARCHIVE / 'index.json').read_bytes())}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('retain', 'verify'))
    {'retain': retain, 'verify': verify}[parser.parse_args().mode]()
