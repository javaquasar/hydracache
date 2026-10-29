"""Offline transport audit only. No process launch, UAC or WSL entry point."""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-root-namespace-uac-attempt-20261023'
INDEX_SHA = '792b54905fe6a1ebdeddc7f9eaf7963c2bb3b93c488610fe6737eb41f9e2a0d0'
WSL = r'C:\Windows\System32\wsl.exe'
WINDOWS = [300, 60, 10]
FALSE_FLAGS = ('admission', 'product', 'server', 'original_owner_run', 'clock_change',
               'service_change', 'tracing_state_changed', 'retry')
UPSTREAM = 'https://github.com/microsoft/WSL/blob/642331364dda7a3d88bf64acb87e7056918a5fc9/src/windows/common/WslClient.cpp#L1367-L1413'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def load(folder=ARCHIVE, expected_sha=INDEX_SHA):
    raw = (folder / 'index.json').read_bytes()
    if digest(raw) != expected_sha:
        raise ValueError('Changed input index')
    index = json.loads(raw)
    if (index['schema'] != 'hc074-root-uac-evidence-v1'
            or index['original_windows'] != WINDOWS
            or any(index[k] is not False for k in FALSE_FLAGS)):
        raise ValueError('Different evidence scope')
    files = {}
    for name, entry in index['files'].items():
        sha = entry['sha256']
        if not re.fullmatch('[0-9a-f]{64}', sha):
            raise ValueError('Unsafe object digest')
        content = (folder / 'objects' / sha).read_bytes()
        if digest(content) != sha or len(content) != entry['bytes']:
            raise ValueError('Changed input object')
        files[name] = content
    return files


def require_process_completion(transport):
    """Fail before dispatch: a console relay is not a tracked guest process."""
    if transport == 'wsl-debug-shell':
        raise ValueError('Debug console has no guest exit/retirement contract; dispatch refused')
    raise ValueError('Unreviewed transport; dispatch refused without fallback')


def assess(stdout, client_exit, timed_out):
    """Classify framing, client status and guest status as independent facts."""
    if type(client_exit) is not int or type(timed_out) is not bool:
        raise ValueError('Explicit client receipt required')
    if timed_out and client_exit != 124:
        raise ValueError('Timeout must retain the original synthetic exit 124')
    # Normalize only in memory, never remove terminal control characters or
    # search substrings that could accept an echoed command as a real marker.
    lines = stdout.replace(b'\r\n', b'\n').replace(b'\r', b'\n').splitlines()
    begin, end = b'HC074_ROOT_METADATA_BEGIN', b'HC074_ROOT_METADATA_END'
    complete = (lines.count(begin) == lines.count(end) == 1
                and lines.index(begin) < lines.index(end))
    return {'stdout_bytes': len(stdout), 'standalone_framing_complete': complete,
            'client_exit': client_exit, 'client_timed_out': timed_out,
            'client_exchange_success': client_exit == 0 and not timed_out,
            'guest_exit_code': None, 'guest_retirement_confirmed': None,
            'process_completion_contract_supported': False,
            'framing_is_not_metadata_or_identity_validation': True,
            'dispatch_allowed': False}


def audit(files):
    def record(name):
        return json.loads(files[name])

    receipt = record('journal/lookup/debug-shell.json')
    invocation = record('journal/subprocess-04-intent.json')
    error = record('journal/subprocess-04-error.json')
    if (receipt['argv'] != [WSL, '--debug-shell']
            or invocation != {'argv': receipt['argv'], 'retry': False, 'timeout': 30}
            or receipt['timeout_seconds'] != 30 or receipt['exit'] != 124
            or receipt['retry'] is not False or error['retry'] is not False
            or not error['error'].startswith('TimeoutExpired(')):
        raise ValueError('Different first exchange or client limit')
    if (record('request/intent.json')['original_windows'] != WINDOWS
            or record('journal/lookup/intent.json')['original_windows'] != WINDOWS):
        raise ValueError('Different original windows')
    # The pinned collection script and all of its guards are untouched.
    script = ROOT / 'tools/wsl-clock-trace-074/root_namespace.py'
    if digest(script.read_bytes()) != '99e140b530e2200d475e7b97c31ece6681563d1ea89bf3c49e589717a62687e5':
        raise ValueError('Original collector changed')
    return {'schema': 'hc074-offline-debug-transport-audit-v1',
            'input_index_sha256': INDEX_SHA,
            'observed': assess(files['journal/lookup/debug-shell.stdout.log'], receipt['exit'], True),
            'upstream_source': UPSTREAM,
            'upstream_release_tag': '2.5.10',
            'upstream_source_is_not_installed_binary_attestation': True,
            'new_wsl_dispatches': 0, 'original_windows': WINDOWS, **dict.fromkeys(FALSE_FLAGS, False)}


if __name__ == '__main__':
    print(json.dumps(audit(load()), indent=2, sort_keys=True))
