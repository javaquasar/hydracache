"""Offline semantic lock delta: remove only the original old-zbus test edge."""
import tomllib


def need(ok, why):
    if not ok:
        raise ValueError(why)


def packages(raw):
    data = tomllib.loads(raw.decode())
    need(data['version'] == 4, 'known complete lock format')
    rows = data['package']
    result = {(r['name'], r['version'], r.get('source')): r for r in rows}
    need(len(result) == len(rows), 'unique package identities')
    return result


def edges(row, catalog):
    result = set()
    for text in row.get('dependencies', []):
        parts = text.split(' ', 2)
        selected = [key for key in catalog if key[0] == parts[0]
                    and (len(parts) == 1 or key[1] == parts[1])
                    and (len(parts) < 3 or '(' + str(key[2]) + ')' == parts[2])]
        need(len(selected) == 1, 'unambiguous complete dependency: ' + text)
        result.add(selected[0])
    need(len(result) == len(row.get('dependencies', [])), 'no duplicate dependency edge')
    return result


def review(before, after):
    old, new = packages(before), packages(after)
    need(not set(new) - set(old), 'no unreviewed registry/package additions')
    roots = [k for k in old if k[0] == 'hydracache-long-run-supervisor-074']
    need(len(roots) == 1 and roots[0] in new, 'one unchanged tool identity')
    removed_edge = [k for k in old if k[0] == 'zbus' and k[1] == '5.19.0']
    need(len(removed_edge) == 1 and removed_edge[0] not in new, 'old zbus completely absent')
    need(not any(k[0] == 'zbus' and k[1].startswith('5.') for k in new), 'no old replacement edge')
    for key, row in new.items():
        expected = old[key]
        need({k: v for k, v in row.items() if k != 'dependencies'} ==
             {k: v for k, v in expected.items() if k != 'dependencies'},
             'same source/version/checksum/metadata for every retained package')
        prior_edges = edges(expected, old)
        if key == roots[0]:
            need(removed_edge[0] in prior_edges, 'exact old root dependency')
            prior_edges = prior_edges - {removed_edge[0]}
        need(edges(row, new) == prior_edges, 'only old root dependency may disappear')
    return dict(old_packages=len(old), new_packages=len(new),
        removed=[dict(name=k[0], version=k[1], source=k[2]) for k in sorted(set(old)-set(new))],
        old_zbus_absent=True, new_registry_nodes=False, retained_checksums_unchanged=True,
        unchanged_semantic_edges_except_old_root_test_dependency=True,
        root_product_changed=False, admission=False)
