"""Export retained B0/C1 source for review; never build or run archived code."""
import json
from pathlib import PurePosixPath
import verify as v


def main():
    index=json.loads((v.OUT/'index.json').read_bytes())
    def get(digest):return (v.OUT/'objects'/digest).read_bytes()
    result=v.validate(index,get)
    out=v.a.ROOT/'target/chitchat-context-source-review-20261030'
    if out.parent.resolve()!=v.a.ROOT.resolve()/'target':raise ValueError('exact workspace target parent')
    out.mkdir(exist_ok=False)
    exported={}
    for name,row in index['members'].items():
        if not name.startswith(('fixtures/B0/','fixtures/C1/')):continue
        relative=PurePosixPath(name.removeprefix('fixtures/'))
        if relative.is_absolute() or any(p in ('.','..') for p in relative.parts) or '\\' in name:
            raise ValueError('canonical contained source member')
        path=out.joinpath(*relative.parts)
        if not path.resolve().is_relative_to(out.resolve()):raise ValueError('source export containment')
        path.parent.mkdir(parents=True,exist_ok=True)
        with path.open('xb') as stream:stream.write(get(row['sha256']))
        if v.a.hashlib.sha256(path.read_bytes()).hexdigest()!=row['sha256']:raise ValueError('fresh exported source')
        exported[relative.as_posix()]=row
    with (out/'SOURCE_REVIEW_ONLY.json').open('x',encoding='utf8',newline='\n') as stream:
        json.dump(dict(index_sha256=v.a.hashlib.sha256((v.OUT/'index.json').read_bytes()).hexdigest(),
            members=exported,decision=result['decision'],root_adopted=False,release_admitted=False,
            archived_code_executed=False,build_or_measurement_dispatched=False),stream,indent=2,sort_keys=True)
    print(json.dumps(dict(destination=str(out),members=len(exported),archived_code_executed=False)))


if __name__=='__main__':main()
