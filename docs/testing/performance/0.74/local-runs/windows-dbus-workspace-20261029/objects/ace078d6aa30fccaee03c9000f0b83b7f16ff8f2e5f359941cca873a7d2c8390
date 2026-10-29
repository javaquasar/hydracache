"""Source-audited follow-up for the exact old zbus Tokio/tracing feature effect.

The original strict audit and first refusal remain unchanged. This does not run
Cargo again or introduce a generic dependency-edge exemption.
"""
import hashlib
import tomllib

import lock_review as core

OLD_SHA = 'e7d3f2da900f6d134b59397e46885600ba9cd395e6249e388fc6e687d301b19e'
TOKIO_SHA = '6e6a777fce6800a5ed2a0efb672d7751445556ee40d258157938ba4506b78c35'
QUALIFIED_SHA = 'd2836489dcf6772e84e0fdbe124078ee6fd968e99a53d312f21f10c0bfe222af'
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
TOKIO = ('tokio', '1.53.3', REGISTRY)
TRACING = ('tracing', '0.1.44', REGISTRY)
ZBUS5 = ('zbus', '5.19.0', REGISTRY)


def review(before, after, old_manifest, tokio_manifest, qualified_manifest, candidate_manifest):
    sha = lambda raw: hashlib.sha256(raw).hexdigest()
    core.need(sha(old_manifest) == OLD_SHA and sha(tokio_manifest) == TOKIO_SHA
              and sha(qualified_manifest) == QUALIFIED_SHA, 'exact primary source feature proof')
    feature = b'hc074-unix-fixture = []\n'
    core.need(candidate_manifest.count(feature) == 1
              and candidate_manifest.replace(feature, b'', 1) == qualified_manifest,
              'candidate manifest changes only the empty nondefault fixture feature')
    old_src = tomllib.loads(old_manifest.decode())
    new_src = tomllib.loads(candidate_manifest.decode())
    tokio_src = tomllib.loads(tokio_manifest.decode())
    core.need(old_src['package']['name'] == 'zbus' and old_src['package']['version'] == '5.19.0'
              and 'tracing' in old_src['dependencies']['tokio']['features']
              and new_src['package']['version'] == '6.1.0'
              and 'tracing' not in new_src['dependencies']['tokio']['features'],
              'only old backend explicitly enables Tokio tracing')
    core.need(tokio_src['package']['name'] == 'tokio' and tokio_src['package']['version'] == '1.53.3'
              and tokio_src['target']['cfg(tokio_unstable)']['dependencies']['tracing']['optional'] is True,
              'pinned Tokio optional cfg(tokio_unstable) tracing dependency')
    old, new = core.packages(before), core.packages(after)
    core.need(not set(new)-set(old), 'no new registry identity')
    roots = [k for k in old if k[0] == 'hydracache-long-run-supervisor-074']
    core.need(len(roots) == 1 and roots[0] in new, 'one original tool')
    core.need(ZBUS5 in old and ZBUS5 not in new and TOKIO in old and TOKIO in new
              and TRACING in old and TRACING not in new, 'exact observed feature graph boundary')
    core.need(not any(k[0] == 'zbus' and k[1].startswith('5.') for k in new), 'old backend absent')
    for key, row in new.items():
        previous = old[key]
        core.need({k:v for k,v in row.items() if k != 'dependencies'} ==
                  {k:v for k,v in previous.items() if k != 'dependencies'},
                  'unchanged retained package version/source/checksum/metadata')
        expected = core.edges(previous, old)
        if key == roots[0]:
            core.need(ZBUS5 in expected, 'old root test edge')
            expected = expected-{ZBUS5}
        if key == TOKIO:
            core.need(TRACING in expected, 'old unified Tokio tracing feature')
            expected = expected-{TRACING}
        core.need(core.edges(row, new) == expected, 'no unrelated dependency effect')
    return dict(old_packages=len(old), new_packages=len(new),
        removed=[dict(name=k[0],version=k[1],source=k[2]) for k in sorted(set(old)-set(new),
            key=lambda k:(k[0],k[1],k[2] or ''))],
        old_zbus_absent=True, no_new_packages=True, retained_checksums_unchanged=True,
        allowed_effects=['old root zbus 5 test dependency', 'source-proven Tokio optional tracing dependency'],
        source_manifests_sha256=dict(zbus5=sha(old_manifest),tokio=sha(tokio_manifest),
            qualified_zbus6=sha(qualified_manifest),candidate_zbus6=sha(candidate_manifest)),
        original_strict_audit_refusal_preserved=True, new_cargo_run=False,
        root_product_changed=False, admission=False)
