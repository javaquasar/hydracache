"""Read-only package inventory and old replay review; does NOT clear execution effects."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tomllib


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def audit(root):
    root = root.resolve(strict=True)
    manifest_raw = (root / 'crates/xtask/Cargo.toml').read_bytes()
    manifest = tomllib.loads(manifest_raw.decode())
    require(manifest['package']['name'] == 'xtask', 'package identity')
    require(manifest['package'].get('autotests', True) and manifest['package'].get('autobins', True), 'default discovery')
    require(not any(k in manifest for k in ('lib', 'bin', 'test')), 'custom target discovery')
    files = set((root / 'crates/xtask/src').rglob('*.rs')) | set((root / 'crates/xtask/tests').rglob('*.rs'))
    pending = sorted(files)
    edges, missing = [], []
    while pending:
        path = pending.pop()
        text = path.read_text(encoding='utf-8')
        for literal in re.findall(r'^\s*#\[path\s*=\s*"([^"\n]+)"\s*\]', text, re.M):
            target = (path.parent / literal).resolve()
            require(target.is_relative_to(root), 'source edge outside repository')
            edge = dict(source=path.relative_to(root).as_posix(), literal=literal,
                        target=target.relative_to(root).as_posix())
            edges.append(edge)
            if not target.is_file():
                missing.append(edge)
            elif target not in files:
                files.add(target)
                pending.append(target)
    sources = {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(files)}
    targets = ['unittests src/lib.rs', 'unittests src/main.rs'] + [
        'tests/' + p.name for p in sorted((root / 'crates/xtask/tests').glob('*.rs'))]
    pattern = re.compile(r'Command::new|\.spawn\(|\.output\(|\.status\(|include!|systemctl|systemd-run|ssh|wsl|clock_settime|timedatectl')
    sightings = []
    for path in sorted(files):
        for number, line in enumerate(path.read_text(encoding='utf-8').splitlines(), 1):
            if pattern.search(line):
                sightings.append(dict(source=path.relative_to(root).as_posix(), line=number, text=line))
    review_path = root / 'tools/xtask-package-attribution-074/review.py'
    spec = importlib.util.spec_from_file_location('sealed_package_log_review', review_path)
    review = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(review)
    old = review.analyze(root)
    return dict(targets=targets, target_count=len(targets), sources=sources, literal_path_candidates=edges,
        unresolved_literal_path_candidates=missing, lexical_sightings=sightings,
        scanner_scope='lexical candidates including strings/comments; NOT expanded Rust AST or a reachability proof',
        unchanged_command=review.ARGV, outer_bound_seconds=780, disk_floor_bytes=4*1024**3,
        old_replays=old, package_replayed=False, execution_effects_cleared=False,
        manifest_sha256=hashlib.sha256(manifest_raw).hexdigest(),
        scope='local Windows default package, not --all-features, --ignored, workspace verify or Linux live tests')


if __name__ == '__main__':
    print(json.dumps(audit(Path(__file__).resolve().parents[2]), indent=2, sort_keys=True))
