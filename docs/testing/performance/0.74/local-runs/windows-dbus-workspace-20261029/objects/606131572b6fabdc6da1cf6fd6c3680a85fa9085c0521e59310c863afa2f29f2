"""Read-only review of the exact registry-vendored Swagger input; never extract."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import stat
import tarfile
import zipfile

CRATE_SHA = '552a64d6a0d251ae28bc021838800943cce60c9ec2e60ff331f54e79f696268f'
ZIP_SHA = 'b3c07e091559b59a833f66547eb1fc18f2896f96e3f1f953e2f1efb328aa3394'
PREFIX = 'swagger-ui-5.32.6'
MAX_ARCHIVE = 32 * 1024**2
MAX_TOTAL = 64 * 1024**2
MAX_FILE = 16 * 1024**2
MAX_ENTRIES = 10_000
REQUIRED = {'index.html', 'swagger-ui.css', 'swagger-ui-bundle.js',
            'swagger-ui-standalone-preset.js', 'swagger-initializer.js'}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def inventory(raw, prefix=PREFIX):
    """Independent structural/CRC review; does not model a filesystem extractor."""
    require(len(raw) <= MAX_ARCHIVE, 'archive size')
    members = {}
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        entries = archive.infolist()
        require(0 < len(entries) <= MAX_ENTRIES, 'entry count')
        require(sum(row.file_size for row in entries) <= MAX_TOTAL, 'expanded total')
        spellings = set()
        for row in entries:
            name = row.orig_filename
            require('\x00' not in name and '\\' not in name and ':' not in name, 'ambiguous path')
            parts = name.removesuffix('/').split('/')
            require(parts[0] == prefix and all(p and p not in ('.', '..') and p == p.rstrip(' .')
                                              for p in parts), 'noncanonical path')
            key = '/'.join(parts)
            require(key.casefold() not in spellings, 'duplicate or case alias')
            spellings.add(key.casefold())
            mode = row.external_attr >> 16
            kind = stat.S_IFMT(mode)
            require(kind in (0, stat.S_IFREG, stat.S_IFDIR), 'symlink or special file')
            require(not mode & 0o7000, 'privileged mode bits')
            require(kind != stat.S_IFDIR or row.is_dir(), 'directory type mismatch')
            require(not row.flag_bits & 1, 'encrypted entry')
            require(row.compress_type in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED), 'compression method')
            require(row.file_size <= MAX_FILE, 'file size')
            # Reading every entry checks actual decompressed length and CRC.
            content = archive.read(row)
            require(len(content) == row.file_size, 'expanded length')
            members[key] = dict(bytes=len(content), sha256=sha(content),
                                mode=mode, directory=row.is_dir())
    for key in members:
        parts = key.split('/')
        for end in range(1, len(parts)):
            parent = members.get('/'.join(parts[:end]))
            require(parent is None or parent['directory'], 'file/directory conflict')
    require(all(prefix+'/dist/'+name in members and not members[prefix+'/dist/'+name]['directory']
                for name in REQUIRED), 'required UI assets')
    return members


def review(raw):
    require(len(raw) <= MAX_ARCHIVE and sha(raw) == CRATE_SHA, 'exact registry crate checksum')
    name = 'utoipa-swagger-ui-vendored-0.2.0/res/v5.32.6.zip'
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        entries = [row for row in archive.getmembers() if row.name == name]
        require(len(entries) == 1 and entries[0].isfile() and entries[0].size <= MAX_ARCHIVE,
                'one original regular ZIP member')
        data = archive.extractfile(entries[0]).read()
    require(sha(data) == ZIP_SHA, 'exact original UI ZIP checksum')
    members = inventory(data)
    return dict(crate_sha256=CRATE_SHA, zip_sha256=ZIP_SHA, members=members,
                entry_count=len(members), expanded_bytes=sum(r['bytes'] for r in members.values()),
                files_extracted=False, archived_code_executed=False,
                general_untrusted_extraction_safety_claimed=False, release_admitted=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('crate', type=Path)
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    with (args.out/'intent.json').open('x') as stream:
        json.dump(dict(source_sha256=sha(Path(__file__).read_bytes()), read_only=True,
                       crate_sha256=CRATE_SHA, zip_sha256=ZIP_SHA), stream, indent=2)
    try:
        require(args.crate.is_file() and not args.crate.is_symlink() and not args.crate.is_junction(), 'input custody')
        require(args.crate.stat().st_size <= MAX_ARCHIVE, 'input size')
        result = review(args.crate.read_bytes())
        with (args.out/'result.json').open('x') as stream:
            json.dump(result, stream, indent=2, sort_keys=True)
        print(json.dumps(dict(entry_count=result['entry_count'], expanded_bytes=result['expanded_bytes'], files_extracted=False)))
    except BaseException as error:
        with (args.out/'refusal.json').open('x') as stream:
            json.dump(dict(type=type(error).__name__, reason=str(error)), stream, indent=2)
        raise
