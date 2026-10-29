"""Bounded lexical inventory of original path/include literals, never a Rust gate."""
import json,re
from pathlib import Path

def inventory(root,package,require):
 root=root.resolve();manifest=root/package
 queue=list((manifest/'src').rglob('*.rs'))+list((manifest/'tests').rglob('*.rs'));seen=set();result={};edges=[]
 pattern=r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|[A-Za-z_][A-Za-z_0-9]*|[^\s]'
 while queue:
  source=queue.pop().resolve()
  if source in seen:continue
  seen.add(source);require(source.is_relative_to(root) and source.is_file() and not source.is_symlink(),'regular original source inventory')
  tokens=[m.group() for m in re.finditer(pattern,source.read_text(encoding='utf8'),re.S) if not m.group().startswith(('//','/*'))]
  for i,t in enumerate(tokens):
   target=None;code=False
   if tokens[i:i+4]==['#','[','path','='] and i+5<len(tokens) and tokens[i+4].startswith('"') and tokens[i+5]==']':
    target=source.parent/json.loads(tokens[i+4]);code=True
   elif t in ('include','include_str','include_bytes') and tokens[i+1:i+3]==['!','(']:
    depth=1;j=i+3
    while j<len(tokens) and depth:
     depth+=(tokens[j]=='(')-(tokens[j]==')');j+=1
    require(depth==0,'balanced original include expression');args=tokens[i+3:j-1]
    if len(args)==1 and args[0].startswith('"'):target=source.parent/json.loads(args[0])
    elif args[:6]==['concat','!','(','env','!','(']:
     require(args[6:9]==['"CARGO_MANIFEST_DIR"',')',','] and args[-1]==')','only declared original manifest concat')
     parts=[json.loads(v) for v in args[9:-1] if v!=','];require(all(isinstance(v,str) for v in parts),'literal concat suffix')
     target=Path(str(manifest)+''.join(parts))
    else:raise ValueError('unclassified original include expression: '+str(source))
    code=t=='include'
   if target is not None:
    target=target.resolve();require(target.is_relative_to(root) and target.is_file() and not target.is_symlink(),'closed original include/path target: '+str(target))
    name=target.relative_to(root).as_posix();result[name]=target.read_bytes();edges.append(dict(source=source.relative_to(root).as_posix(),target=name,code=code))
    if code:queue.append(target)
 return result,edges
