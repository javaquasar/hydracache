"""Separate first ABBA/BAAB cohort for the uninstalled streaming candidate."""
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import time
import capture


def review(raw):
    lines=raw.decode().splitlines()
    rows=[json.loads(line[len('HC074_CANONICAL '):]) for line in lines if line.startswith('HC074_CANONICAL ')]
    done=[json.loads(line[len('HC074_CANONICAL_DONE '):]) for line in lines if line.startswith('HC074_CANONICAL_DONE ')]
    expected=dict(blocks=6,samples=72,warmup=False,allocator_interposition=False,installed=False,product=False,package_replay=False)
    capture.require(done==[expected] and all(type(done[0][k]) is type(v) for k,v in expected.items()) and len(rows)==72,'complete cohort')
    groups={}
    for i,row in enumerate(rows):
        block,rest=divmod(i,12)
        path,slot=divmod(rest,4)
        order=(False,True,True,False) if block%2==0 else (True,False,False,True)
        capture.require(type(row.get('block')) is int and type(row.get('slot')) is int and type(row.get('candidate')) is bool,'identity types')
        capture.require((row['block'],row['slot'],row.get('path'),row['candidate'])==(block,slot,capture.PATHS[path],order[slot]),'fixed order')
        capture.require(all(type(row.get(k)) is int and row[k]>=0 for k in ('wall_ns','parent_cpu_ns','iterations','accepted')),'sample numbers')
        capture.require(row['iterations']==row['accepted']==32,'workload/results')
        groups.setdefault(row['path']+'/'+('candidate' if row['candidate'] else 'baseline'),[]).append(row)
    summary={key:{metric:dict(samples=len(group),median=statistics.median(row[metric]/32 for row in group),
        mean=statistics.mean(row[metric]/32 for row in group),total=sum(row[metric] for row in group))
        for metric in ('wall_ns','parent_cpu_ns')} for key,group in groups.items()}
    return dict(samples=rows,per_call=summary,independent_process_replicates=False,allocation_measured=False,
        installed=False,accepted_for_integration=False,package_replayed=False,product_candidate=False)


def run(build,out):
    out.mkdir(parents=True,exist_ok=False)
    def write(name,value):
        with (out/name).open('x',encoding='utf-8',newline='\n') as stream:
            json.dump(value,stream,indent=2,sort_keys=True);stream.write('\n')
    write('intent.json',dict(blocks=6,paths=3,samples=72,iterations=32,timeout_seconds=180,
        disk_floor_bytes=4*1024**3,retry=False,warmup=False,installed=False,server=False,services=False,clocks=False))
    process=None
    try:
        capture.require(os.name=='nt' and shutil.disk_usage(capture.ROOT).free>=4*1024**3,'platform/disk')
        capture.require(not any(key.startswith(('HC074_','RUST_TEST_')) for key in os.environ),'fixture overrides')
        result=json.loads((build/'result.json').read_bytes())
        source=json.loads((build/'intent.json').read_bytes())['source']
        capture.require(result['exit']==0 and result['source_unchanged'] and result['head_unchanged'],'build receipt')
        for name,digest in source.items():
            if name.endswith(('.rs','.toml','.lock')):
                capture.require(capture.sha((capture.ROOT/name).read_bytes())==digest,'compiled source drift')
        original='tools/authority-storage-source-074/contract.rs'
        capture.require(capture.sha((capture.ROOT/original).read_bytes())=='aefcd4f6a9ed0acc1e072225ea3ed4dda82d2d1ccda0ba4f22ca74459efa64c2','installed baseline changed')
        records=[json.loads(line) for line in (build/'stdout.log').read_text().splitlines() if line.startswith('{')]
        artifact,=[row for row in records if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')=='authority_context_074']
        binary=Path(artifact['executable']).resolve()
        capture.require(binary.parent==(capture.ROOT/'target/debug/deps').resolve(),'binary path')
        paths=[]
        for name in capture.PATHS:
            before=subprocess.check_output(['git','show',capture.BASELINE+':'+name],cwd=capture.ROOT)
            raw=(capture.ROOT/name).read_bytes()
            capture.require(before.replace(b'\r\n',b'\n')==raw.replace(b'\r\n',b'\n'),'committed input drift')
            paths.append(dict(path=name,sha256=capture.sha(raw)))
        write('pins.json',dict(paths=paths,production_authority=False))
        write('preflight.json',dict(compiled_source=source,binary_sha256=capture.sha(binary.read_bytes())))
        with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
            process=subprocess.Popen([str(binary),'--exact','normalization_candidate_child','--nocapture','--test-threads=1'],
                cwd=capture.ROOT,stdout=stdout,stderr=stderr,env=dict(os.environ,HC074_CANONICAL_INPUT=str((out/'pins.json').resolve())))
            started=time.monotonic()
            while process.poll() is None:
                capture.require(time.monotonic()-started<180,'outer timeout')
                capture.require(shutil.disk_usage(capture.ROOT).free>=4*1024**3,'disk floor')
                try:process.wait(timeout=1)
                except subprocess.TimeoutExpired:pass
        capture.require(process.returncode==0,'first child refused')
        capture.require(all(capture.sha((capture.ROOT/name).read_bytes())==digest for name,digest in source.items()),'source changed')
        write('result.json',review((out/'stdout.log').read_bytes()))
        print(json.dumps(dict(samples=72,measured_child_repeated=False,installed=False)))
    except Exception as error:
        write('refusal.json',dict(type=type(error).__name__,reason=str(error),exit=None if process is None else process.poll()))
        if process is not None and process.poll() is None:
            with (out/'cleanup.log').open('xb') as log:
                subprocess.run(['taskkill','/PID',str(process.pid),'/T','/F'],stdout=log,stderr=subprocess.STDOUT,timeout=30,check=False)
            process.wait(timeout=30)
        raise


if __name__=='__main__':run(Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve())
