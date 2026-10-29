"""Read-only review of Rust includes and the original owned-case caller mode."""
from pathlib import Path
import json
import re
import subprocess

import common as c


def main():
    review = json.loads((c.BASE/'freeze/runtime-reviewed.json').read_bytes())
    c.guard(review)
    extra = set()
    missing = []
    for name in review['source']:
        if not name.startswith(c.PKG+'/') or not name.endswith('.rs'):
            continue
        path = c.ROOT/name
        text = path.read_text(encoding='utf8')
        members = re.findall(r'#\[path\s*=\s*"([^"]+)"\]', text)
        members += re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"', text)
        for member in members:
            source = (path.parent/member).resolve()
            if not source.is_file():
                missing.append([name, member])
                continue
            relative = source.relative_to(c.ROOT).as_posix()
            if relative not in review['source']:
                extra.add(relative)
    original = review['source_snapshot']['head']
    files = {}
    for name in sorted(extra):
        raw = (c.ROOT/name).read_bytes()
        blob = subprocess.check_output(['git','show',original+':'+name], cwd=c.ROOT)
        c.need(raw == blob or b'\r\n' not in blob and raw == blob.replace(b'\n', b'\r\n'),
               'unchanged original shared Rust input, with explicit checkout newline view')
        files[name] = dict(raw_sha256=c.sha(raw), base_blob_sha256=c.sha(blob), base_commit=original)
    out = c.BASE/'caller-compatibility-review'
    out.mkdir(exist_ok=False)
    result = dict(extra=files, unresolved=missing,
        original_caller='owned_case::selected_alone recognizes exactly four argv entries',
        old_exact_driver_bypasses_original_wrapper=True,
        correction='add --color=never to case dispatch; unfiltered serial integration harnesses',
        product_source_changed=False, windows_changed=False, release_admitted=False)
    c.write(out, 'result.json', result)
    for name in extra:
        (out/Path(name).name).write_bytes((c.ROOT/name).read_bytes())
    print(json.dumps(result))


if __name__ == '__main__':
    main()
