"""One exact compatible Markdown successor; every historical proof remains live."""
import copy
import os

import dbus_inputs as d
import security_lock
import swagger_inputs as s
import yaml_inputs as y

CURRENT = '34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c'
PREVIOUS = d.BODIES['Cargo.lock'][0]
OLD = b'''name = "pulldown-cmark"
version = "0.13.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e9f068eba8e7071c5f9511831b44f32c740d5adf574e990f946ddb53db2f314e"'''
NEW = OLD.replace(b'0.13.4', b'0.13.5').replace(
    b'e9f068eba8e7071c5f9511831b44f32c740d5adf574e990f946ddb53db2f314e',
    b'278e9268c93b9e498a2da46c34d90ba93a3255eb99009646398f34bc73ab36eb')


def predecessor(raw):
    raw = raw.replace(b'\r\n', b'\n')  # Original inherited normalization.
    s.require(s.sha(raw) == CURRENT and raw.count(NEW) == 1, 'unreviewed complete Markdown input')
    raw = raw.replace(NEW, OLD)
    s.require(s.sha(raw) == PREVIOUS, 'complete pre-Markdown lock')
    return raw


def reviewed_inputs(root, data):
    s.require(not any(k.startswith('SWAGGER_UI_') for k in os.environ), 'unreviewed Swagger build override')
    s.require(data['auxiliary']['Cargo.lock'] == security_lock.PREVIOUS
              and data['auxiliary']['Cargo.toml'] == s.BODIES['Cargo.toml'][1]
              and data['auxiliary']['crates/xtask/Cargo.toml'] == y.BODIES['crates/xtask/Cargo.toml'][1],
              'original inherited Markdown predecessors')
    result = copy.deepcopy(data)
    for name in ('Cargo.lock','Cargo.toml','crates/xtask/Cargo.toml'):
        path = root/name
        s.require(path.is_file() and not path.is_symlink() and not path.is_junction()
                  and path.resolve() == root.resolve()/name, 'Markdown input custody')
        raw = path.read_bytes()
        if name == 'Cargo.lock':
            previous = y.predecessor(name, d.predecessor(name, predecessor(raw)))
            security_lock.predecessor(s.predecessor(name, previous))
            current = CURRENT
        elif name == 'Cargo.toml':
            s.predecessor(name, d.predecessor(name, raw))
            current = d.BODIES[name][0]
        else:
            y.predecessor(name, raw)
            current = y.BODIES[name][0]
        result['auxiliary'][name] = current
    return result
