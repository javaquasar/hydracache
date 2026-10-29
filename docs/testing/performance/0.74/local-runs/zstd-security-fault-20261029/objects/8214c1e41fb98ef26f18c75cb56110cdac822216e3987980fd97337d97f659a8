"""Independent held-byte, unapplied fileset and transitive publication guards."""
import copy
import json
import unittest

import publication as p


class Retained(unittest.TestCase):
    def test_all_packet_bytes_and_unapplied_changeset_preimages_are_exact(self):
        raw = (p.PACKET / 'index.json').read_bytes()
        self.assertEqual(p.r.sha(raw), p.INDEX)
        packet = json.loads(raw)
        self.assertEqual(len(packet['files']), 436)
        for name in packet['files']:
            p.member(name)
        recipe = json.loads(p.member('candidate/fileset.json'))
        self.assertFalse(recipe['applied'] or recipe['root_adoption_admitted'] or recipe['compiler_gate_complete'])
        operations = recipe['operations']
        self.assertEqual(len(operations), 291)
        self.assertEqual(len({v['path'] for v in operations}), 291)
        self.assertEqual(sum(v['before_sha256'] is None for v in operations), 268)
        for row in operations:
            self.assertEqual(p.r.sha(p.member('candidate/after/' + row['path'])), row['after_sha256'])
            if row['before_sha256'] is not None:
                self.assertEqual(p.r.sha(p.member('candidate/before/' + row['path'])), row['before_sha256'])
        self.assertFalse(packet['root_changed'] or packet['compilation_executed'] or packet['server'])
        first = json.loads(p.member('effects/refusal.json'))
        self.assertFalse(first['passed'] or first['cargo_repeated'] or first['result_emitted'])

    def test_actual_full_graph_published_roots_cannot_reach_tool_or_vendor(self):
        metadata = json.loads(p.member('metadata-all-features/cargo.stdout.json'))
        value = p.review(metadata)
        self.assertEqual(value['publishable_roots'], 23)
        self.assertEqual(value['transitive_reachable'], 599)
        self.assertEqual(value['published_vendor_intersections'], [])
        original = copy.deepcopy(metadata)
        packages = {v['id']: v for v in metadata['packages']}
        start = next(k for k in metadata['workspace_members'] if packages[k]['publish'] != [])
        vendor = next(k for k in packages if packages[k]['name'] == 'zbus')
        node = next(n for n in metadata['resolve']['nodes'] if n['id'] == start)
        node['deps'].append(dict(pkg=vendor, name='hidden-vendor', dep_kinds=[]))
        with self.assertRaises(ValueError):
            p.review(metadata)
        self.assertNotEqual(original, metadata)

    def test_both_original_metadata_owners_retire_without_root_or_locked_source_changes(self):
        for phase, count in (('metadata', 678), ('metadata-all-features', 683)):
            execution = json.loads(p.member(phase + '/execution.json'))
            result = json.loads(p.member(phase + '/result.json'))
            self.assertTrue(execution['retired'] and result['passed'])
            self.assertFalse(execution['errors'] or result['root_changed'])
            self.assertEqual(execution['exit'], 0)
            self.assertEqual(result['packages'], count)
            self.assertEqual(result['workspace_members'], 36)
        self.assertTrue(json.loads(p.member('metadata-all-features/result.json'))['source_unchanged'])


if __name__ == '__main__':
    unittest.main()
