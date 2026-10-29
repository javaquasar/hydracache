"""Pure source-archive reconstruction. No archive code is imported or executed."""
from pathlib import PurePosixPath
import hashlib
import io
import json
import tarfile


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2) + '\n').encode()


def safe_name(name):
    require(isinstance(name, str) and name and '\\' not in name and ':' not in name,
            'portable relative name')
    require(all(ord(char) >= 32 and ord(char) != 127 for char in name), 'no control characters')
    require(not name.startswith('/') and all(p not in ('', '.', '..') for p in name.split('/')),
            'exact relative components')
    require(str(PurePosixPath(name)) == name, 'no normalized aliases')
    return name


def unpack(raw, prefix, canonical=False):
    """Read into memory only; refuse links, aliases and duplicate file names."""
    safe_name(prefix)
    result = {}
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:*') as archive:
        for entry in archive:
            name = safe_name(entry.name.rstrip('/') if entry.isdir() else entry.name)
            require(name == prefix or name.startswith(prefix + '/'), 'one archive prefix')
            if entry.isdir():
                require(not canonical, 'canonical archives contain only files')
                continue
            require(entry.isfile() and name != prefix, 'regular source member only')
            rel = name[len(prefix) + 1:]
            require(rel not in result, 'duplicate source member')
            if canonical:
                require(entry.uid == entry.gid == entry.mtime == 0
                        and entry.mode == 0o644 and not entry.uname and not entry.gname
                        and not entry.linkname
                        and set(entry.pax_headers).issubset({'path'}), 'canonical metadata')
            body = archive.extractfile(entry).read()
            require(len(body) == entry.size, 'complete source member')
            result[rel] = body
    require(result, 'nonempty source archive')
    return result


def pack(members, prefix):
    """Uncompressed PAX tar: fixed ordering/metadata, no compressor/clock input."""
    safe_name(prefix)
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode='w', format=tarfile.PAX_FORMAT) as archive:
        for name, body in sorted(members.items()):
            safe_name(name)
            info = tarfile.TarInfo(prefix + '/' + name)
            info.size = len(body)
            info.mode = 0o644
            info.uid = info.gid = info.mtime = 0
            info.uname = info.gname = ''
            archive.addfile(info, io.BytesIO(body))
    raw = output.getvalue()
    require(unpack(raw, prefix, True) == members, 'canonical inverse')
    return raw


def inventory(members):
    return {name: sha(raw) for name, raw in sorted(members.items())}


def rebuild(recipe, archives, objects):
    require(recipe['schema'] == 'hc074-zbus-source-package-v1', 'known recipe')
    require(set(archives) == set(recipe['bases']) == {'zbus', 'zbus_macros', 'zbus_utils'},
            'exact base packages')
    final, packages = {}, {}
    changes = recipe['changes']
    require(len({(r['crate'], r['path']) for r in changes}) == len(changes), 'unique changes')
    required_objects = {r['after_sha256'] for r in changes}
    require(set(objects) == required_objects, 'exact replacement objects')
    require(all(sha(raw) == digest for digest, raw in objects.items()), 'replacement bytes')
    for crate, row in sorted(recipe['bases'].items()):
        require(sha(archives[crate]) == row['sha256'], 'exact upstream archive')
        prefix = crate + '-' + row['version']
        members = unpack(archives[crate], prefix)
        require(inventory(members) == row['members'], 'whole upstream inventory')
        for change in changes:
            if change['crate'] != crate:
                continue
            name = safe_name(change['path'])
            before = members.get(name)
            require((sha(before) if before is not None else None) == change['before_sha256'],
                    'exact change preimage')
            members[name] = objects[change['after_sha256']]
        require(inventory(members) == recipe['final_members'][crate], 'whole final inventory')
        final[crate] = members
        packages[crate] = pack(members, prefix)
    require(set(r['crate'] for r in changes).issubset(final), 'no foreign change owner')
    require({name: sha(raw) for name, raw in packages.items()} == recipe['source_archive_sha256'],
            'exact canonical archive identities')
    return packages, final
