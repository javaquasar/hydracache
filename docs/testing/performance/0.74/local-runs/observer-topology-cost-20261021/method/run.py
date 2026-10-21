"""One bounded ordinary-user WSL session; fixed local diagnostic binaries only."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import time

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("capture",ROOT/"tools/local-profile-session-074/runner.py")
capture=importlib.util.module_from_spec(spec);spec.loader.exec_module(capture)
PACKET=ROOT/"docs/testing/performance/0.74/local-runs/observer-topology-cost-20261021"
IMAGES={lane:f"/home/artur/.cache/hydracache-074-observer-control-{lane}/release/observer-topology-profile-074" for lane in ("194","188")}


def git(*args):
    return subprocess.run(["/mnt/c/Program Files/Git/cmd/git.exe","--no-optional-locks","-C","C:/Workspace/prj/jq/cashe/hydracache-074",*args],check=True,capture_output=True,text=True,timeout=10).stdout.strip()


def context():
    files=[Path(__file__),Path(capture.__file__),PACKET/"input.json",ROOT/"docs/testing/performance/0.74/qualification-manifest.toml",*map(Path,IMAGES.values())]
    return dict(source=git("rev-parse","HEAD"),tracked_dirty=git("status","--porcelain","--untracked-files=no"),
                boot=Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
                pid=os.getpid(),uid=os.getuid(),start_ticks=Path("/proc/self/stat").read_text().rsplit(")",1)[1].split()[19],
                hashes={str(p):capture.sha(p) for p in files},
                config={str(p):capture.sha(p) if p.exists() else None for p in map(Path,("/etc/wsl.conf","/etc/systemd/timesyncd.conf","/mnt/c/Users/ArturBuzov/.wslconfig"))})


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--source",required=True);args=parser.parse_args()
    capture.require(not(PACKET/"metadata.json").exists() and not(PACKET/"result.json").exists(),"Never resume a session")
    started=time.monotonic();cases=[];failure=None;initial=None
    capture.write_new(PACKET/"session-intent.json",dict(source=args.source,cases=12,child_seconds=60,session_seconds=300,server=False,product=False))
    try:
        initial=context();capture.write_new(PACKET/"metadata.json",initial)
        capture.require(initial["source"]==args.source and not initial["tracked_dirty"] and initial["uid"]!=0,"Source/user drift")
        preparation=capture.read_json(PACKET/"prepare.json")
        capture.require(preparation["source"]==args.source,"Preparation source drift")
        for lane in ("194","188"):
            capture.require(capture.sha(IMAGES[lane])==preparation["images"][lane],"Image drift")
            for case in range(6):
                capture.require(time.monotonic()-started<230,"Insufficient session budget")
                capture.require(context()==initial,"Metadata drift")
                prefix=PACKET/f"{lane}-{case}"
                capture.write_new(prefix.with_suffix(".intent.json"),dict(lane=lane,case=case,source=args.source))
                receipt=capture.capture(["/usr/bin/timeout","--signal=TERM","--kill-after=5s","60s",IMAGES[lane],"--input",str(PACKET/"input.json"),"--case",str(case),"--source",args.source],
                    {"PATH":"/usr/bin:/bin"},prefix.with_suffix(".log"),min(started+280,time.monotonic()+65))
                receipt.update(source=args.source,lane=lane,case=case)
                capture.write_new(prefix.with_suffix(".json"),receipt);cases.append(receipt)
                capture.require(context()==initial,"Metadata drift")
                if receipt["exit"]!=0:break
    except BaseException as error:
        failure=type(error).__name__+": "+str(error)
    finally:
        try:
            final=context();capture.write_new(PACKET/"postflight.json",final)
            capture.require(final==initial,"Postflight drift")
        except BaseException as error:
            failure=(failure or "")+" Postflight: "+str(error)
        result=dict(source=args.source,cases=cases,failure=failure,complete=not failure and len(cases)==12 and all(c["exit"]==0 for c in cases),server=False,product=False,native_gate_open=False)
        capture.write_new(PACKET/"result.json",result)
    print(json.dumps(result))
    return 0 if result["complete"] else 1


if __name__=="__main__":
    def interrupted(number,_frame):
        raise InterruptedError(f"Session signal {number}; no continuation")
    signal.signal(signal.SIGTERM,interrupted)
    signal.signal(signal.SIGHUP,interrupted)
    raise SystemExit(main())
