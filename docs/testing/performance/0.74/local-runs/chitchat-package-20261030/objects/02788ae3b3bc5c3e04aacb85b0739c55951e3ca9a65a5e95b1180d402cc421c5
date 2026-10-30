"""Eight exact reversible syntax rewrites; no algorithm/default/threshold change."""
import json
from pathlib import Path
RULES=json.loads(Path(__file__).with_name('style-rules.json').read_bytes())
def change(files,reverse=False):
    out=dict(files)
    rows=RULES['sites']+RULES.get('format_sites',[])
    for row in reversed(rows) if reverse else rows:
        old,new=(row['after'],row['before']) if reverse else (row['before'],row['after'])
        old,new=old.encode(),new.encode();name=row['path']
        if out[name].count(old)!=1:raise ValueError('one exact reviewed style preimage: '+name)
        out[name]=out[name].replace(old,new)
    return out
