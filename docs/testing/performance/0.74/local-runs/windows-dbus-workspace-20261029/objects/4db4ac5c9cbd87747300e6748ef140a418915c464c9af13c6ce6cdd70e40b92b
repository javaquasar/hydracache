"""Actual all-feature transitive publication closure, including registry edges."""
from pathlib import Path
import json

import review as r

PACKET = r.ROOT / 'docs/testing/performance/0.74/local-runs/dbus-root-integration-20261028'
INDEX = '149698f228bba92ece4c5454db982977093899bfc8da6cf5e50c9405c2708ca9'


def member(name):
    raw = (PACKET / 'index.json').read_bytes()
    r.need(r.sha(raw) == INDEX, 'exact held root review packet')
    row = json.loads(raw)['files'][name]
    raw = (PACKET / 'objects' / row['sha256']).read_bytes()
    r.need(r.sha(raw) == row['sha256'] and len(raw) == row['bytes'], 'held actual metadata bytes')
    return raw


def review(metadata):
    packages = {p['id']: p for p in metadata['packages']}
    nodes = {p['id']: p for p in metadata['resolve']['nodes']}
    workspace = set(metadata['workspace_members'])
    r.need(len(packages) == len(metadata['packages']) == 683 and len(workspace) == 36
           and set(nodes) == set(packages), 'complete exact all-feature source graph')
    roots = {p for p in workspace if packages[p]['publish'] != []}
    vendor = {p for p in packages if packages[p]['source'] is None and p not in workspace}
    r.need(len(roots) == 23 and len(vendor) == 3 and {(packages[p]['name'], packages[p]['version']) for p in vendor} ==
           {('zbus', '6.1.0'), ('zbus_macros', '6.1.0'), ('zbus_utils', '6.0.0')}, 'actual source publication boundary')
    tools = {p for p in workspace if packages[p]['name'] == 'hydracache-long-run-supervisor-074'}
    r.need(len(tools) == 1 and not tools & roots, 'one actual unpublished supervisor')
    reached = set(roots)
    todo = list(roots)
    while todo:
        for dep in nodes[todo.pop()]['deps']:
            child = dep['pkg']
            r.need(child in packages, 'closed actual transitive edge')
            if child not in reached:
                reached.add(child)
                todo.append(child)
    r.need(not reached & (vendor | tools), 'no published-root path reaches private tool or local backport')
    return dict(workspace_members=36, publishable_roots=len(roots), transitive_reachable=len(reached),
        vendor_packages=3, published_vendor_intersections=[], all_dependency_kinds_and_targets=True,
        no_published_private_override=True, common_registry_updates_still_require_root_gates=True,
        registry_publication=False, root_adopted=False, release_admitted=False)


if __name__ == '__main__':
    raw = member('metadata-all-features/cargo.stdout.json')
    value = review(json.loads(raw))
    value.update(input_packet_sha256=INDEX, metadata_sha256=r.sha(raw))
    out = r.ROOT / 'docs/testing/performance/0.74/dbus-root-publication-review-20261028.json'
    with out.open('x', encoding='utf8', newline='\n') as stream:
        stream.write(json.dumps(value, indent=2, sort_keys=True)+'\n')
    print(json.dumps(value))
