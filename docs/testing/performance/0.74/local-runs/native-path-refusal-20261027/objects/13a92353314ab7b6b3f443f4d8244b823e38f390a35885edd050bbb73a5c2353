"""Transfer checks only. A native checkout never grants test/release admission."""
import hashlib
import json
from pathlib import PurePosixPath
import re

FLOOR = 4 * 1024**3
SLACK = 512 * 1024**2
QUAL = 'docs/testing/performance/0.74/qualification-manifest.toml'
QUAL_SHA = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs)


def oid(value):
    return isinstance(value, str) and re.fullmatch('[0-9a-f]{40}', value) is not None


def safe_path(name):
    if not isinstance(name, str):
        return False
    path = PurePosixPath(name)
    return (bool(name) and bool(path.parts) and not path.is_absolute()
            and path.as_posix() == name and '\\' not in name and ':' not in name
            and not any(part in ('.', '..') or part.lower() == '.git' for part in path.parts)
            and not any(ord(c) < 32 or ord(c) == 127 for c in name))


def tree(raw):
    require(raw.endswith(b'\0'), 'unterminated tree')
    rows = {}
    for item in raw[:-1].split(b'\0'):
        meta, name = item.split(b'\t', 1)
        name = name.decode('utf-8')
        mode, kind, blob, size = meta.decode('ascii').split()
        require(safe_path(name) and name not in rows, 'unsafe or duplicate tree path')
        require(mode in ('100644', '100755') and kind == 'blob' and oid(blob), 'nonregular tree entry')
        require(size.isdecimal(), 'invalid blob size')
        rows[name] = dict(mode=mode, blob=blob, bytes=int(size))
    require(rows, 'empty tree')
    return dict(sorted(rows.items()))


def refs(raw):
    result = {}
    for line in raw.decode('ascii').splitlines():
        value, name = line.split(' ', 1)
        require(oid(value) and name.startswith('refs/') and safe_path(name) and name not in result,
                'unsafe or duplicate ref')
        result[name] = value
    require(result, 'empty refs')
    return dict(sorted(result.items()))


def space(free, blob_bytes, bundle_bytes):
    required = FLOOR + SLACK + blob_bytes + 2 * bundle_bytes
    require(free >= required, 'physical backing volume capacity')
    return required


def request(value):
    require(set(value) == {'schema', 'source', 'tree', 'refs', 'bundle_sha256', 'bundle_bytes', 'qualifier_sha256'},
            'request fields')
    require(value['schema'] == 'hc074-native-frontier-transfer-v1' and oid(value['source']), 'request identity')
    require(value['qualifier_sha256'] == QUAL_SHA, 'qualification drift')
    require(re.fullmatch('[0-9a-f]{64}', value['bundle_sha256']) is not None
            and type(value['bundle_bytes']) is int and value['bundle_bytes'] > 0, 'bundle identity')
    require(value['tree'] and value['refs'], 'empty transfer inventory')
    for name, row in value['tree'].items():
        require(safe_path(name) and set(row) == {'mode', 'blob', 'bytes'} and oid(row['blob'])
                and row['mode'] in ('100644', '100755') and type(row['bytes']) is int and row['bytes'] >= 0,
                'tree row')
    require(all(name.startswith('refs/') and safe_path(name) and oid(blob) for name, blob in value['refs'].items()),
            'ref row')
    return value


def git_environment(environment, null='/dev/null'):
    # Preparation only. No inherited hook/filter/askpass/transport/config helper.
    clean = {k: v for k, v in environment.items() if not k.upper().startswith('GIT_')}
    clean.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=null, GIT_TERMINAL_PROMPT='0',
                 GIT_CONFIG_SYSTEM=null, GIT_LFS_SKIP_SMUDGE='1')
    return clean


def blob_digest(raw):
    return hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
