"""Verify then export all three source controls; never build or run them."""
import json
from pathlib import PurePosixPath
import verify as v


def main():
    raw=(v.OUT/'index.json').read_bytes();index=json.loads(raw)
    v.validate(index,lambda h:(v.OUT/'objects'/h).read_bytes())
    out=v.r.ROOT/'target/chitchat-call-source-review-20261030'
    v.r.need(out.parent.resolve()==v.r.ROOT.resolve()/'target','exact workspace export parent');out.mkdir(exist_ok=False)
    exported={}
    for name,row in index['members'].items():
        if not name.startswith(('fixtures/B0/','fixtures/T1/','fixtures/C2/')):continue
        relative=PurePosixPath(name.removeprefix('fixtures/'))
        v.r.need(not relative.is_absolute() and not any(n in ('.','..') for n in relative.parts) and '\\' not in name,'canonical source member')
        path=out.joinpath(*relative.parts);v.r.need(path.resolve().is_relative_to(out.resolve()),'contained fresh source export')
        path.parent.mkdir(parents=True,exist_ok=True);body=(v.OUT/'objects'/row['sha256']).read_bytes()
        with path.open('xb') as f:f.write(body)
        v.r.need(v.r.sha(path.read_bytes())==row['sha256'],'fresh whole exported source');exported[relative.as_posix()]=row
    with (out/'SOURCE_REVIEW_ONLY.json').open('x',encoding='utf8',newline='\n') as f:
        json.dump(dict(index_sha256=v.r.sha(raw),members=exported,root_adopted=False,release_admitted=False,
            archived_producer_executed=False,build_dispatched=False),f,indent=2,sort_keys=True)
    print(json.dumps(dict(destination=str(out),members=len(exported),build_dispatched=False)))


if __name__=='__main__':main()
