"""Bounded deterministic commit-prefix fixture, not a Git caller or pin authority."""
import hashlib
import json
import sys


def candidate(raw, ordinal):
    return raw + b'\nhc074-collision-' + str(ordinal).encode('ascii') + b'\n'


def oid(raw):
    return hashlib.sha1(b'commit ' + str(len(raw)).encode('ascii') + b'\0' + raw).hexdigest()


def collide(raw, prefix, limit=1 << 20):
    if (not raw or len(raw) > 65536 or len(prefix) != 4
            or any(char not in '0123456789abcdef' for char in prefix)
            or not 1 <= limit <= 1 << 20):
        raise ValueError('bounded synthetic commit and four-digit prefix required')
    for ordinal in range(limit):
        body = candidate(raw, ordinal)
        digest = oid(body)
        if digest.startswith(prefix):
            return dict(body_hex=body.hex(), oid=digest, ordinal=ordinal,
                        limit=limit, seed='hc074-collision', authority=False)
    raise ValueError('first bounded collision search exhausted')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('one four-digit prefix required')
    raw = sys.stdin.buffer.read(65537)
    print(json.dumps(collide(raw, sys.argv[1]), sort_keys=True))
