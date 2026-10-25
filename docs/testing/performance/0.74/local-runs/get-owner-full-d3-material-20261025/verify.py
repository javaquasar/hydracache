"""Offline byte/invariant replay. No host contact, workload or admission."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys

HERE = Path(__file__).resolve().parent
PREFIX = 'docs/testing/performance/0.74/local-runs/get-owner-full-d3-material-20261025/'


def require(value, message):
    if not value:
        raise ValueError(message)


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON field')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs)


def verify(read):
    index = decode(read('index.json'))
    require(index['schema_version'] == 1, 'index schema')
    for key in ['measured_cohort_started', 'd3_complete', 'admission', 'product_or_threshold_changes']:
        require(index[key] is False, 'closed boundary: ' + key)
    members = {}
    for name, item in index['members'].items():
        require('..' not in name.split('/') and not name.startswith('/'), 'member path')
        digest = item['sha256']
        require(re.fullmatch('[0-9a-f]{64}', digest) is not None, 'digest shape')
        raw = read('objects/' + digest)
        require(type(item['bytes']) is int and 0 <= item['bytes'] <= 16 * 1024**2, 'byte bound')
        require(len(raw) == item['bytes'] and hashlib.sha256(raw).hexdigest() == digest, 'raw digest: ' + name)
        members[name] = raw

    def doc(name): return decode(members[name])
    def server(group, name): return doc('server/' + group + '/' + name)
    def passed_log(name):
        raw = members[name].decode()
        require('FAILED' not in raw, 'failed success log')
        return sum(map(int, re.findall(r'test result: ok\. (\d+) passed', raw)))

    build = 'get-owner-d3-build-first'
    result = server(build, 'result.json')
    require(result['passed'] is True and result['source_unchanged'] is True and result['workload_started'] is False, 'baseline build')
    require(len(server(build, 'source-manifest.json')) == 2572, 'baseline source inventory')
    require([c['name'] for c in result['cases']] == ['locked-fetch','build-off','build-on','check','clippy'], 'build order')
    require(all(c['exit'] == 0 for c in result['cases']), 'build command failed')
    images = doc('server/image-export.json')
    for variant, features in [('off', []), ('on', ['--features', 'get-owner'])]:
        image = images['timing-' + variant]
        receipt = server(build, 'image-' + variant + '.json')
        require(image['sha256'] == receipt['sha256'] and image['bytes'] == receipt['bytes'], 'image export')
        require(receipt['features'] == features and receipt['allocation_diagnostics'] is False and receipt['executed'] is False, 'build features/execution')

    first = 'get-owner-material-checks-first'
    require(server(first, 'result.json')['passed'] is False, 'first refusal lost')
    require(server(first, 'off-material-result.json')['exit'] == 101, 'first refusal status')
    require(b'CryptoProvider' in members['server/' + first + '/off-material.stdout.log'], 'first refusal cause')
    corrected = 'get-owner-material-checks-corrected'
    corrected_result = server(corrected, 'result.json')
    require(corrected_result['passed'] is True and corrected_result['product_fixture_started'] is False, 'corrected scope/result')
    require(len(corrected_result['cases']) == 8 and all(c['exit'] == 0 for c in corrected_result['cases']), 'corrected commands')
    linux = []
    for variant in ['off', 'on']:
        count = sum(passed_log('server/' + corrected + '/' + variant + '-' + part + '.stdout.log') for part in ['material','arguments','metadata-cli'])
        require(count == 6, 'Linux metadata count')
        linux.append(count)
    for group in [build, first, corrected, 'get-owner-provider-control-first']:
        require(server(group, 'preflight.json') == server(group, 'postflight.json'), 'host frame changed')
        frame = server(group, 'postflight.json')
        require(frame['markers_absent'] is True and frame['installed_unchanged'] is True, 'host predicates')
        require(frame['supervisor'] == 'MainPID=8839\nNRestarts=0\nActiveState=active\nSubState=running\n', 'supervisor identity changed')

    windows = []
    require(doc('windows/get-owner-fixed-material-first/result.json')['exit'] == 101, 'Windows first refusal lost')
    for variant in ['off', 'on']:
        prefix = 'windows/get-owner-fixed-material-all-' + variant + '/'
        result = doc(prefix + 'result.json')
        require(result['exit'] == 0 and result['source_unchanged'] is True and result['head_unchanged'] is True, 'Windows source/test gate')
        count = passed_log(prefix + 'stdout.log')
        require(count == 89, 'Windows standalone count')
        windows.append(count)
    for phase in ['check', 'clippy']:
        require(doc('windows/get-owner-fixed-material-' + phase + '/result.json')['exit'] == 0, 'Windows ' + phase)
    for phase in ['get-owner-full-d3-doc-check', 'get-owner-full-d3-performance-contract']:
        require(doc('windows/' + phase + '/result.json')['exit'] == 0, 'documentation/contract gate')

    provider = server('get-owner-provider-control-first', 'control.stdout.log')
    require(provider['provider_capability_control_passed'] is True and provider['admission'] is False and provider['performance_claim'] is False, 'provider claim')
    require(provider['synthetic_control_only'] is True and len(provider['samples']) == 5, 'provider scope')
    for i, sample in enumerate(provider['samples']):
        require(sample['phase'] == i and sample['stable'] is True and sample['covered'] is True, 'provider phase')
        require(sample['allocator_rw_mapping_bytes'] == sample['arena'] + sample['mmap'], 'mapping conservation')
        require(sample['allocator_accounted_in_use'] + sample['free_bins_and_top'] == sample['arena'] + sample['mmap'], 'chunk conservation')
        require(sample['allocator_mapping_rss_bytes'] <= sample['allocator_rw_mapping_bytes'] <= sample['allocator_address_bytes'], 'mapping sizes')
        require(sample['arena_maps'] == 0, 'not a multi-arena proof')
    require(provider['samples'][4]['free_bins_and_top'] > provider['samples'][0]['free_bins_and_top'], 'retained-free positive control')
    inventory = doc('transport/get-owner-memory-capability-inventory/stdout.log')
    require(inventory['provider_version'] == '2.39' and inventory['allocator_counter_measurement'] is False, 'provider inventory scope')
    return dict(schema_version=1, windows_standalone_off_on=windows, linux_metadata_off_on=linux,
                baseline_images={name: item['sha256'] for name, item in images.items()},
                synthetic_provider_phases=5, provider_multiarena_proven=False,
                measured_d3_attempts=0, full_d3_complete=False, candidate_accepted=False,
                thresholds_changed=False, supervisor_stopped=False, raw_members=len(members))


def main():
    if sys.argv[1:] == ['--staged']:
        read = lambda name: subprocess.check_output(['git','cat-file','blob',':' + PREFIX + name])
    else:
        require(not sys.argv[1:], 'expected optional --staged')
        read = lambda name: (HERE / name).read_bytes()
    print(json.dumps(verify(read), indent=2, sort_keys=True))


if __name__ == '__main__':
    main()
