"""Source-only module closure with explicit declaration contexts; no execution gate.

The old scanner and its pins remain unchanged. A file reached through #[path]
has a different child directory from an ordinary foo.rs module. Never select
the first existing directory when both declaration contexts disagree.
"""
import hashlib
from pathlib import Path


def module_edges(root, document):
    root = root.resolve(strict=True)
    rows = {row['source']: row for row in document['sources']}
    if len(rows) != len(document['sources']):
        raise ValueError('duplicate module source')
    styles = {name: set() for name in rows}
    for name in rows:
        path = Path(name)
        if (path.parent == Path('crates/xtask/src') and path.name in ('lib.rs', 'main.rs')) or (
                path.parent == Path('crates/xtask/tests') and path.suffix == '.rs'):
            styles[name].add('root')

    def candidates(name, site):
        source = root / name
        parts = site['scope'].split('::')
        if not all(part and part.isidentifier() for part in parts):
            raise ValueError('unsupported module scope')
        detail = site['detail']
        if detail.startswith('literal:'):
            return [[source.parent.joinpath(*parts[:-1], detail[8:])]]
        if detail != 'default-module-path':
            raise ValueError('unsupported module declaration')
        bases = set()
        for style in styles[name]:
            bases.add(source.parent if style in ('root', 'literal') or source.name == 'mod.rs'
                      else source.with_suffix(''))
        return [[base.joinpath(*parts).with_suffix('.rs'), base.joinpath(*parts, 'mod.rs')]
                for base in sorted(bases)]

    def resolve(name, site):
        groups = candidates(name, site)
        resolved = set()
        for group in groups:
            for path in group:
                if not path.resolve().is_relative_to(root):
                    raise ValueError('source edge outside repository')
            existing = [path.resolve() for path in group if path.is_file()]
            if len(existing) != 1:
                return None, groups
            resolved.add(existing[0])
        return (next(iter(resolved)) if len(resolved) == 1 else None), groups

    sites = [(name, site) for name, row in rows.items() for site in row['sites']
             if site['kind'] == 'module']
    # Discover declaration contexts to a finite fixed point, not by choosing a
    # filesystem fallback. Every distinct context must resolve the same edge.
    for _ in range(2 * len(rows) + 1):
        changed = False
        for name, site in sites:
            target, _ = resolve(name, site)
            if target is None:
                continue
            relative = target.relative_to(root).as_posix()
            style = 'literal' if site['detail'].startswith('literal:') else 'ordinary'
            if relative in styles and style not in styles[relative]:
                styles[relative].add(style)
                changed = True
        if not changed:
            break
    else:
        raise ValueError('module context closure bound')

    additions, unresolved = {}, []
    for name, site in sites:
        target, groups = resolve(name, site)
        if target is None:
            unresolved.append(dict(source=name, scope=site['scope'],
                                   contexts=sorted(styles[name]),
                                   candidates=[[str(path) for path in group] for group in groups]))
            continue
        relative = target.relative_to(root).as_posix()
        if relative not in rows:
            additions[relative] = hashlib.sha256(target.read_bytes()).hexdigest()
    return additions, unresolved
