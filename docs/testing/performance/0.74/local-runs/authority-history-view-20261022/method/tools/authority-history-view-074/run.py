"""One source-identified offline replay; output publication is exclusive."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile

import view_inputs as view

ROOT=Path(__file__).resolve().parents[2]
OUTPUT=ROOT/'target/authority-history-view-first'


def materialize(repo,directory,m):
    archive=directory/'source.zip';root=directory/'view';root.mkdir()
    paths=sorted(m['source_files'])+[view.BASE+'/'+name for name in m['packets']]
    with archive.open('xb') as output:
        result=subprocess.run(['git','archive','--format=zip',view.SOURCE,*paths],cwd=repo,stdout=output,stderr=subprocess.PIPE,timeout=60,check=False)
    view.need(result.returncode==0,'Frozen Git archive failed: '+result.stderr.decode(errors='replace'))
    view.need(archive.stat().st_size<=m['bounds']['archive_bytes'],'Git archive byte bound')
    tree=view.extract(archive,root,m)
    return root,dict(sha256=view.sha(archive.read_bytes()),bytes=archive.stat().st_size,tree=tree)


def worker(root,m):
    result=subprocess.run([sys.executable,'-I','-B',str(Path(__file__).with_name('worker.py')),str(root)],
        input=json.dumps(m).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=root,timeout=60,check=False)
    view.need(result.returncode==0,'Historical worker refused: '+result.stderr.decode(errors='replace'))
    parsed=view.decode(result.stdout);view.need(parsed['history_only'] is True and parsed['historical_facts_equal'] is True,'Historical worker scope drift')
    return parsed,result.stdout,result.stderr


def first(output):
    view.need(Path(output).resolve()==OUTPUT.resolve(),'Only separate first replay output allowed')
    parent=ROOT/'target';view.need(not parent.is_symlink() and not parent.is_junction(),'Linked scratch parent');parent=parent.resolve()
    view.need(not OUTPUT.exists(),'Never replace a first historical replay')
    OUTPUT.mkdir();failure=None;receipt={}
    try:
        m=view.manifest()
        current=view.current_fixture_identity(ROOT)
        with tempfile.TemporaryDirectory(dir=parent,prefix='ahv-') as temporary:
            generated=Path(temporary).resolve()
            view.need(generated.parent==parent and generated.name.startswith('ahv-'),'Unsafe generated cleanup target')
            root,archive=materialize(ROOT,generated,m);actual,stdout,stderr=worker(root,m)
            with (OUTPUT/'worker.stdout.json').open('xb') as stream:stream.write(stdout)
            with (OUTPUT/'worker.stderr.log').open('xb') as stream:stream.write(stderr)
            view.need(view.verify_tree(root,m)==archive['tree'],'Final historical view drift')
            receipt=dict(archive=archive,worker=actual,current_baseline=current,
                worker_stdout_sha256=view.sha(stdout),worker_stderr_sha256=view.sha(stderr))
        view.need(not generated.exists(),'Generated view was not retired')
        receipt['generated_view_removed']=True
    except Exception as error:failure=type(error).__name__+': '+str(error)
    receipt.update(schema='authority-history-view-first-074-v1',history_source=view.SOURCE,
        tooling_source=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        manifest_sha256=view.INPUT_SHA,failure=failure,model_images_executed=0,runtime_candidate_implemented=False,
        runtime_candidate_guard_implemented=False,original_owner_profiles=0,server=False,product=False)
    with (OUTPUT/'result.json').open('x',encoding='utf-8',newline='\n') as stream:json.dump(receipt,stream,indent=2,sort_keys=True);stream.write('\n')
    view.need(failure is None,'First historical replay refused: '+str(failure))
    return receipt


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    first(args.output)
    print('Historical reader replay passed; current runtime candidate admission remains closed')
