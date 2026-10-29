"""Complete GNU formatter check, same original subreaper/deadline/capacity owner."""
from pathlib import Path
import json
import os
import sys
import time

import common as c

c.need(len(sys.argv)==2 and sys.argv[1] in ('linux-initial','linux-qualified','final'), 'fixed GNU format check')
action=sys.argv[1]
out=c.BASE/'format'/action
toolchain=Path('/home/artur/.rustup/toolchains/1.94.0-x86_64-unknown-linux-gnu/bin')
env=dict(HOME='/home/artur',USER='artur',LOGNAME='artur',LANG='C.UTF-8',
    PATH=str(toolchain)+':/usr/local/bin:/usr/bin:/bin', CARGO_NET_OFFLINE='true',
    CARGO_HOME='/home/artur/.cargo', RUSTC=str(toolchain/'rustc'),RUSTFMT=str(toolchain/'rustfmt'),
    RUSTUP_TOOLCHAIN='1.94.0-x86_64-unknown-linux-gnu',
    RUSTC_WRAPPER='',RUSTC_WORKSPACE_WRAPPER='',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='2')
argv=[str(toolchain/'cargo'),'fmt','--all','--manifest-path',str(c.ROOT/'Cargo.toml'),'--','--check']
deadline=time.monotonic()+1100
result=dict(passed=False,server=False,services=False,clocks_changed=False,compilation_executed=False)
try:
    c.need(os.getuid()==os.getgid()==1000, 'original nonroot local Linux')
    for name in ('cargo','cargo-fmt','rustfmt','rustc'):c.need((toolchain/name).is_file(),'qualified GNU '+name)
    watch=c.ROOT/'scripts/ci/watchdog_linux_group.py'
    c.need(c.sha(watch.read_bytes())=='e5ff52835fc2486171dbdbbb1b6df47177f583b4539f561779992c2c5065a691',
           'original Linux CI process owner')
    group=c.load(watch,'root_package_formatter_group')
    c.write(out,'native-intent.json',dict(argv=argv,environment=env,deadline=deadline,
        bound=1200,session_bound=1100,floor=4*1024**3,poll=5,retirement=5,retry=False))
    os.environ.clear();os.environ.update(env)
    execution=c.owned()._owned(group.LinuxGroup(),argv,str(c.ROOT),c.capacity,bound=1200,deadline=deadline)
    c.write(out,'native-execution.json',execution)
    c.need(execution['retired'] and not execution['errors'],'original GNU format retirement')
    result.update(passed=execution['exit']==0,exit=execution['exit'])
except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
c.write(out,'native-result.json',result)
raise SystemExit(result.get('exit',125))
