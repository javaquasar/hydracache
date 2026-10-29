"""Separate exact-source parser requested-heap control; no CPU/live rerun."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
PRIOR = ROOT / 'target/status-field-scan-20261024'
OUT = ROOT / 'target/status-field-memory-20261024'
ARCHIVE = ROOT / 'docs/testing/performance/0.74/local-runs/status-field-scan-20261024'
INDEX_SHA = 'fa6cf67055f7e857ecee141e8ac32972684ca3d8d46558e5352904bac465e0fc'

def sha(raw): return hashlib.sha256(raw).hexdigest()

def archived_sources(lane, archive=ARCHIVE):
    if lane not in ('baseline','candidate'):
        raise ValueError('Exact parser source lane required')
    raw = (archive/'index.json').read_bytes()
    if sha(raw) != INDEX_SHA:
        raise ValueError('Retained parser source index differs')
    index = json.loads(raw)
    def member(name):
        entry = index['files']['observation/'+lane+'/'+name]
        digest = entry['sha256']
        if len(digest)!=64 or any(value not in '0123456789abcdef' for value in digest):
            raise ValueError('Unsafe retained object identity')
        value = (archive/'objects'/digest).read_bytes()
        if sha(value)!=digest or len(value)!=entry['bytes']:
            raise ValueError('Retained source object differs')
        return value
    intent = json.loads(member('intent.json'))
    if json.loads(member('result.json'))['exit']:
        raise ValueError('Retained original build refused')
    names = {'main.rs','projection.rs','projection_groups.rs'}
    if lane=='candidate': names.add('projection_fields.rs')
    if set(intent['inputs']) != names:
        raise ValueError('Original parser source inventory differs')
    values = {name:member(name) for name in names}
    if any(sha(value)!=intent['inputs'][name] for name,value in values.items()):
        raise ValueError('Original source receipt differs')
    return values

def sources(folder):
    if not folder.exists():
        return archived_sources(folder.name)
    intent = json.loads((folder/'intent.json').read_bytes())
    receipt = json.loads((folder/'result.json').read_bytes())
    if receipt['exit'] or sha((folder/'parser').read_bytes()) != receipt['image_sha256']:
        raise ValueError('Original pure-parser build incomplete or changed')
    values = {name: (folder/name).read_bytes() for name in intent['inputs']}
    if any(sha(raw) != intent['inputs'][name] for name, raw in values.items()):
        raise ValueError('Retained original parser input changed')
    return values

def harness(main, probe):
    text = main.decode().replace('\r\n', '\n')
    if text.count('const SHAPES:') != 1 or text.count('#[repr(C)]\nstruct Timespec') != 1:
        raise ValueError('Exact dataset/context boundaries required')
    text = text.split('#[repr(C)]\nstruct Timespec', 1)[0]
    for old in ('use std::time::Instant;\n', 'const ITERATIONS: usize = 20_000;\n'):
        if text.count(old) != 1: raise ValueError('Only unused timing plumbing may be removed')
        text = text.replace(old, '', 1)
    if 'clock_gettime' in text or 'fn main(' in text:
        raise ValueError('Clock or old entry leaked into resource control')
    return text.encode() + b'\n' + probe

def run(variant):
    if variant not in ('first', 'v2-doc-placement'):
        raise ValueError('Separate reviewed resource-control variant required')
    out = OUT if variant == 'first' else OUT.with_name(OUT.name+'-'+variant)
    out.mkdir()
    results = {}
    failure = None
    try:
        inputs = {lane:sources(PRIOR/lane) for lane in ('baseline','candidate')}
        if inputs['baseline']['main.rs'] != inputs['candidate']['main.rs']:
            raise ValueError('Different original workload harnesses')
        probe = Path(__file__).with_name('probe.rs').read_bytes()
        main = harness(inputs['baseline']['main.rs'], probe)
        for compiler, version in (('194','1.94.0'),('188','1.88.0')):
            transcripts = []
            for lane in ('baseline','candidate'):
                folder = out/(compiler+'-'+lane);folder.mkdir()
                material = dict(inputs[lane]);material['main.rs'] = main
                for name, raw in material.items():
                    with (folder/name).open('xb') as stream:stream.write(raw)
                guest = '/mnt/' + folder.drive[0].lower() + folder.as_posix()[2:]
                prefix = ['wsl.exe','-d','Ubuntu','-u','artur','--cd','/','--exec',
                          '/usr/bin/timeout','--signal=TERM','--kill-after=2s']
                build = prefix + ['120s','/usr/bin/env','RUSTUP_AUTO_INSTALL=0',
                    '/home/artur/.cargo/bin/rustc','+'+version,'--edition=2021','-C','opt-level=3',
                    '-C','codegen-units=1','-D','warnings',guest+'/main.rs','-o',guest+'/probe']
                execute = prefix + ['20s','/usr/bin/env','-i','PATH=/usr/bin:/bin',guest+'/probe']
                intent = dict(build=build,execute=execute,inputs={name:sha(raw) for name,raw in material.items()},
                    cpu_measurement=False,live=False,server=False,services=False,clocks=False,admission=False,retry=False)
                (folder/'intent.json').write_text(json.dumps(intent,indent=2),encoding='utf-8')
                for stage, argv in (('build',build),('run',execute)):
                    with (folder/(stage+'.stdout.log')).open('xb') as stdout,(folder/(stage+'.stderr.log')).open('xb') as stderr:
                        code = subprocess.run(argv,cwd=ROOT,stdout=stdout,stderr=stderr,check=False).returncode
                    (folder/(stage+'-exit.json')).write_text(json.dumps(dict(exit=code)),encoding='utf-8')
                    if code: raise ValueError('First '+compiler+'-'+lane+' '+stage+' refusal retained')
                    if stage == 'build':
                        image = sha((folder/'probe').read_bytes())
                        (folder/'image.json').write_text(json.dumps(dict(sha256=image,bytes=(folder/'probe').stat().st_size)),encoding='utf-8')
                if sha((folder/'probe').read_bytes()) != image:
                    raise ValueError('Pinned allocation image changed')
                if any(sha((folder/name).read_bytes())!=digest for name,digest in intent['inputs'].items()):
                    raise ValueError('Exact allocation inputs changed')
                transcript = (folder/'run.stdout.log').read_bytes()
                if transcript.count(b'case shape=')!=264 or not transcript.endswith(b'complete cases=264 clocks=false procfs=false admission=false product=false rss=false\n'):
                    raise ValueError('Incomplete allocation cohort')
                transcripts.append(transcript)
            if transcripts[0] != transcripts[1]:
                raise ValueError('First '+compiler+' semantic/allocation transcript differs')
            results[compiler]=dict(cases=264,exact_transcript_equal=True,transcript_sha256=sha(transcripts[0]),
                non_regression=True,after_drop_live_bytes=0,rss=False)
    except Exception as error:
        failure = str(error)
    result = dict(failure=failure,results=results,live=False,server=False,clocks=False,services=False,
                  cpu_measurement=False,whole_runtime_memory_claim=False,admission=False,retry=False)
    (out/'result.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result))
    raise SystemExit(int(failure is not None))

if __name__ == '__main__':run(sys.argv[1])
