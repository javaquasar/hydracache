"""Retain exact applied-root package results, including every first refusal."""
from pathlib import Path
import json

import common as c

OLD = c.ROOT/'target/dbus-root-package-20261028'
OUT = c.ROOT/'docs/testing/performance/0.74/local-runs/dbus-root-package-20261028'


def main():
    review = json.loads((c.BASE/'freeze/runtime-reviewed.json').read_bytes())
    c.guard(review)
    for phase in ('format','checks','focus-194','focus-188','spool-194','spool-188','tests-194','tests-188'):
        value = json.loads((c.BASE/phase/'result.json').read_bytes())
        native = json.loads((c.BASE/phase/'native-result.json').read_bytes())
        execution = json.loads((c.BASE/phase/'execution.json').read_bytes())
        c.need(value['passed'] and native['passed'] and native['source_unchanged']
               and native['boot_unchanged'] and execution['exit'] == 0 and execution['retired']
               and not execution['errors'], 'complete independent package phase: '+phase)
    catalogs = [json.loads((c.BASE/('tests-'+compiler)/'catalogs.json').read_bytes())
                for compiler in ('194','188')]
    totals = [json.loads((c.BASE/('tests-'+compiler)/'totals.json').read_bytes())
              for compiler in ('194','188')]
    c.need(catalogs[0] == catalogs[1] and totals[0] == totals[1], 'identical full GNU/MSRV catalogs/accounting')
    raw = {}
    for scope, prefix in ((OLD, 'first-context'), (c.ROOT/'target/dbus-root-package-20261028-custody2', 'serial-context'), (c.ROOT/'target/dbus-root-package-20261028-cases3', 'case-entry-first'), (c.ROOT/'target/dbus-root-package-20261028-caller4', 'caller-first'), (c.ROOT/'target/dbus-root-package-20261029-hash5', 'hash-leaf-first'), (c.ROOT/'target/dbus-root-package-20261029-resource6', 'resource-first'), (c.ROOT/'target/dbus-root-package-20261029-spool7', 'spool-format-first'), (c.BASE, 'formatted8')):
        for path in scope.rglob('*'):
            if path.is_file():
                name = path.relative_to(scope).as_posix()
                if name.startswith('input/archives/'):
                    continue  # All archive checksums and decoded file receipts remain.
                raw[prefix+'/'+name] = path.read_bytes()
    for name in review['source']:
        body = (c.ROOT/name).read_bytes()
        c.need(c.sha(body) == review['source'][name], 'self-contained exact tested source')
        raw['tested-source/'+name] = body
    for path in Path(__file__).parent.iterdir():
        if path.is_file():
            raw['producer/'+path.name] = path.read_bytes()
    OUT.mkdir(exist_ok=False)
    objects = OUT/'objects'
    objects.mkdir()
    members = {}
    for name, body in sorted(raw.items()):
        digest = c.sha(body)
        path = objects/digest
        if not path.exists():
            path.write_bytes(body)
        members[name] = dict(sha256=digest, bytes=len(body))
    result = dict(schema_version=1, files=members, root_applied=True,
        qualified_compilers=['1.94.0','1.88.0'], check=True, strict_clippy=True,
        full_case_isolated_default_catalog=True, monolithic_serial_complete=False, case_processes=4, case_threads=1, test_totals_per_compiler=totals[0],
        private_transport_scenarios_per_compiler=10, full_catalogs_equal=True,
        metadata_targets=90, source_files=len(review['source']), registry_identities=644,
        original_owned_case_frame_preserved=True, integration_harness='unfiltered serial', extra_compile_inputs=13,
        original_windows_seconds=[300,60,10], runtime_live_checks_changed=False,
        original_refusals_retained=True, source_worktree_uncommitted=True,
        complete_gate_reuse=review.get('complete_gate_reuse',{}), partial_case_counts_reused=False,
        frozen_review_sha256=c.sha((c.BASE/'freeze/runtime-reviewed.json').read_bytes()),
        server=False, services=False, clocks_changed=False,
        workspace_gate_complete=False, security_gate_complete=False,
        release_admitted=False, performance_claim=False, binaries_excluded=True,
        first_unrecorded_inert_streams_explicit=True)
    (OUT/'index.json').write_text(json.dumps(result, indent=2, sort_keys=True)+'\n', encoding='utf8', newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text -diff\nindex.json -text\n', encoding='ascii', newline='\n')
    print(json.dumps(dict(members=len(members), objects=len(list(objects.iterdir())),
        index_sha256=c.sha((OUT/'index.json').read_bytes()), tests=totals[0])))


if __name__ == '__main__':
    main()
