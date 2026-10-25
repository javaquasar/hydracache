"""Retain generated raw transcripts, never PKI files or images."""
from pathlib import Path
import base64, hashlib, json

root = Path.cwd()
out = root/'docs/testing/performance/0.74/local-runs/get-owner-full-d3-material-20261025'
out.mkdir(exist_ok=False)
(out/'objects').mkdir()
members = {}

def store(name, data):
    assert name not in members
    digest = hashlib.sha256(data).hexdigest()
    path = out/'objects'/digest
    if path.exists(): assert path.read_bytes() == data
    else: path.write_bytes(data)
    members[name] = dict(sha256=digest, bytes=len(data))

server = root/'target/authorized-server-session-20261025'
labels = ['get-owner-d3-host-preflight','get-owner-d3-build-launch','get-owner-d3-build-export',
          'get-owner-material-checks-launch','get-owner-material-first-export',
          'get-owner-material-corrected-launch','get-owner-material-corrected-export',
          'get-owner-memory-capability-inventory','get-owner-provider-control-launch','get-owner-provider-control-export']
for label in labels:
    path = server/label
    for name in ['intent.json','remote.py','stdout.log','stderr.log','result.json']:
        store('transport/'+label+'/'+name, (path/name).read_bytes())
    result = json.loads((path/'result.json').read_bytes())
    assert result == dict(exit=0, error=None)
    if label.endswith('export'):
        packet = json.loads((path/'stdout.log').read_bytes()); assert packet['complete'] is True
        for name, item in packet['members'].items():
            raw = base64.b64decode(item['base64'], validate=True)
            assert len(raw) == item['bytes'] and hashlib.sha256(raw).hexdigest() == item['sha256']
            store('server/'+name, raw)
        if 'images' in packet:
            store('server/image-export.json', json.dumps(packet['images'], sort_keys=True).encode())

local = root/'target/source-continuation-20261024'
for label in ['get-owner-fixed-material-first', 'get-owner-fixed-material-corrected',
              'get-owner-fixed-material-cli-off', 'get-owner-fixed-material-all-off', 'get-owner-fixed-material-all-on',
              'get-owner-fixed-material-check', 'get-owner-fixed-material-clippy',
              'get-owner-full-d3-doc-check', 'get-owner-full-d3-performance-contract']:
    for name in ['intent.json','stdout.log','stderr.log','result.json']:
        store('windows/'+label+'/'+name, (local/label/name).read_bytes())

index = dict(schema_version=1, purpose='GET-owner full-D3 fixed-material preparation and first refusals',
             measured_cohort_started=False, d3_complete=False, admission=False,
             product_or_threshold_changes=False, members=members)
(out/'index.json').write_text(json.dumps(index, indent=2, sort_keys=True)+'\n', encoding='utf-8')
(out/'.gitattributes').write_text('objects/* -text -diff\n', encoding='ascii')
print(json.dumps(dict(output=str(out), members=len(members), objects=len(list((out/'objects').iterdir())))))
