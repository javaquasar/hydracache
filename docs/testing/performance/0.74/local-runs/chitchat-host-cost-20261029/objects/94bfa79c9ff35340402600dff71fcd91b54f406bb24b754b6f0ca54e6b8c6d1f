"""Fixed current-source verification, never a native measurement replay."""
from pathlib import Path
import json
import subprocess
import sys
import traceback

import common as c

DOCS=('docs/articles/007-optimization-starts-with-measurement.md',
      'docs/testing/performance/0.74/README.md',
      'docs/testing/performance/0.74/release-closure-execution-plan-20261026.md',
      'docs/testing/performance/0.74/chitchat-inplace-result-20261029.md',
      'docs/testing/performance/0.74/chitchat-native-cost-result-20261029.md')
mode=sys.argv[1];c.need(mode in ('controls','format','doc-check','performance-contract'),'fixed current-source check')
out=c.BASE/'final'/mode
rustfmt=Path.home()/'.rustup/toolchains/1.94.0-x86_64-pc-windows-msvc/bin/rustfmt.exe'
argv={'controls':[sys.executable,'-B','-m','unittest','discover','-s',str(c.TOOLS),'-p','test_*.py','-v'],
      'format':[str(rustfmt),'--edition','2024','--check','--config','skip_children=true',str(c.TOOLS/'native_tests.rs'),str(c.BASE/'observer.rs')],
      'doc-check':[str(c.ROOT/'target/debug/xtask.exe'),'doc-check'],
      'performance-contract':[str(c.ROOT/'target/debug/xtask.exe'),'performance-contract-check','--release','0.74']}[mode]
if len(sys.argv)==3:
    c.need(sys.argv[2]=='worker','fixed worker')
    value=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(c.ROOT),c.capacity,bound=1200)
    c.write(out,'execution.json',value);raise SystemExit(value['exit'])
out.mkdir(parents=True,exist_ok=False)
with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
    (out/'producer.py').write_bytes(Path(__file__).read_bytes());result=dict(complete=False,release_admitted=False)
    before=c.source();docs={n:c.sha((c.ROOT/n).read_bytes()) for n in DOCS};head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT)
    image=c.sha(Path(argv[0]).read_bytes())
    c.write(out,'intent.json',dict(argv=argv,source=before,documents=docs,head=head.decode(),tool_sha256=image,retry=False,bound=1200))
    try:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),mode,'worker'],cwd=c.ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
        owned=json.loads((out/'execution.json').read_bytes())
        c.need(child.returncode==owned['exit']==0 and owned['retired'] and not owned['errors'],'complete first gate')
        c.need(before==c.source() and docs=={n:c.sha((c.ROOT/n).read_bytes()) for n in DOCS} and head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT),'source/documents/HEAD drift')
        c.need(image==c.sha(Path(argv[0]).read_bytes()),'held tool image drift');result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result)
print(json.dumps(dict(mode=mode,complete=result['complete'])));raise SystemExit(0 if result['complete'] else 1)
