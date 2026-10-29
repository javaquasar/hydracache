"""Fixed owned offline/document checks; never repeats the stateful cohort."""
from pathlib import Path
import json
import subprocess
import sys
import traceback
import common as c

mode=sys.argv[1];out=c.BASE/'final'/mode
rustfmt=Path.home()/'.rustup/toolchains/1.94.0-x86_64-pc-windows-msvc/bin/rustfmt.exe'
commands={
    'controls':[sys.executable,'-B','-m','unittest','discover','-s',str(c.TOOLS),'-p','test_*.py','-v'],
    'format':[str(rustfmt),'--edition','2024','--check','--config','skip_children=true',str(c.TOOLS/'stateful.rs'),str(c.BASE/'observer.rs'),*[str(p) for p in sorted((c.BASE/'fixtures').glob('*/src/serialize.rs'))],*[str(p) for p in sorted((c.BASE/'fixtures').glob('*/src/delta.rs'))]],
    'doc-check':[str(c.ROOT/'target/debug/xtask.exe'),'doc-check'],
    'performance-contract':[str(c.ROOT/'target/debug/xtask.exe'),'performance-contract-check','--release','0.74'],
}
c.need(mode in commands,'fixed nonmeasurement mode');argv=commands[mode]
if len(sys.argv)==3:
    c.need(sys.argv[2]=='worker','fixed owned worker')
    value=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(c.ROOT),c.capacity,bound=1200)
    c.write(out,'execution.json',value);raise SystemExit(value['exit'])
out.mkdir(parents=True,exist_ok=False)
with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
    (out/'producer.py').write_bytes(Path(__file__).read_bytes());source=c.source();head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT)
    docs={p.relative_to(c.ROOT).as_posix():c.sha(p.read_bytes()) for p in (c.ROOT/'docs/articles/007-optimization-starts-with-measurement.md',c.ROOT/'docs/testing/performance/0.74/README.md',c.ROOT/'docs/testing/performance/0.74/release-closure-execution-plan-20261026.md',c.ROOT/'docs/testing/performance/0.74/chitchat-stateful-result-20261029.md') if p.exists()}
    image=c.sha(Path(argv[0]).read_bytes());result=dict(complete=False,release_admitted=False)
    c.write(out,'intent.json',dict(argv=argv,source=source,documents=docs,head=head.decode(),image_sha256=image,bound=1200,retry=False))
    try:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],cwd=c.ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
        owner=json.loads((out/'execution.json').read_bytes())
        c.need(child.returncode==owner['exit']==0 and owner['retired'] and not owner['errors'],'first complete owned outcome')
        c.need(source==c.source() and all(c.sha((c.ROOT/n).read_bytes())==v for n,v in docs.items()) and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT),'source/docs/HEAD stable')
        c.need(image==c.sha(Path(argv[0]).read_bytes()),'held tool image');result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result)
print(json.dumps(dict(mode=mode,**result)));raise SystemExit(0 if result['complete'] else 1)
