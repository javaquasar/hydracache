"""Exact reviewed sys security successor; no resolver or historical receipt rewrite."""
import copy
import hashlib

PREVIOUS = 'e385c45d47050299da7e79a5133d5ec71ca3ea77f6bafbc36744968dff2d28b5'
CURRENT = '50ec8e7e20a8caebeefdbdde7bbafb36b96257daf1204dcd6dfa8f6c5b7c925a'
OLD = (b'name = "zstd-sys"\nversion = "2.0.16+zstd.1.5.7"\n'
       b'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
       b'checksum = "91e19ebc2adc8f83e43039e79776e3fda8ca919132d68a1fed6a5faca2683748"\n')
NEW = (b'name = "zstd-sys"\nversion = "2.1.2+zstd.1.5.7"\n'
       b'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
       b'checksum = "481ada9c543b2323cbb7bbdbb5249177fbf0188ed959ed2c15e4b22421c84ed3"\n')


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def predecessor(raw):
    # Same LF normalization as the inherited source closure, never a partial hash.
    raw = raw.replace(b'\r\n', b'\n')
    if digest(raw) != CURRENT or raw.count(NEW) != 1:
        raise ValueError('unreviewed complete sys security lock')
    previous = raw.replace(NEW, OLD, 1)
    if digest(previous) != PREVIOUS:
        raise ValueError('complete pre-security lock reconstruction refused')
    return previous


def reviewed_inputs(root, data):
    # No arbitrary refreshed catalog: exactly one independently reviewed auxiliary
    # successor. The inherited reviewer still checks every body and graph edge.
    if data['auxiliary']['Cargo.lock'] != PREVIOUS:
        raise ValueError('unexpected inherited security predecessor')
    path = root / 'Cargo.lock'
    if path.is_symlink() or path.is_junction() or path.resolve().parent != root.resolve():
        raise ValueError('security lock path custody')
    predecessor(path.read_bytes())
    result = copy.deepcopy(data)
    result['auxiliary']['Cargo.lock'] = CURRENT
    return result
