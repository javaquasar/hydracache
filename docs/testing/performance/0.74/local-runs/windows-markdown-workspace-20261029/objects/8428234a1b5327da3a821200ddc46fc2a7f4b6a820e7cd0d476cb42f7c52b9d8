"""Separate contextual closure, retaining v4's immutable scanner and first refusal.

Only the module resolver changes. Build/source receipts, fixed syntax child,
eight-round closure bound, per-child 120-second limit and final raw-source
checks all use the previous capture implementation. This grants no replay.
"""
import hashlib
import json
from pathlib import Path
import sys

import module_context_v5
import syntax_audit as previous

PREVIOUS_SHA256 = 'a1b8045abbee202569ed50fc5df43ad9f633524883a71d590cca6ab0ea547b5e'


def capture(root, build, out):
    body = Path(previous.__file__).read_bytes()
    if hashlib.sha256(body).hexdigest() != PREVIOUS_SHA256:
        raise ValueError('immutable previous syntax scanner changed')
    original = previous.module_edges
    try:
        previous.module_edges = module_context_v5.module_edges
        previous.capture(root, build, out)
    finally:
        previous.module_edges = original
    with (out / 'resolver.json').open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(dict(schema='hc074-contextual-module-capture-v5',
                       previous_scanner_sha256=PREVIOUS_SHA256,
                       resolver_sha256=hashlib.sha256(Path(module_context_v5.__file__).read_bytes()).hexdigest(),
                       declaration_contexts=True, fallback_search=False,
                       package_dispatched=False, execution_effects_cleared=False), stream,
                  indent=2, sort_keys=True)
        stream.write('\n')


if __name__ == '__main__':
    capture(Path(__file__).resolve().parents[2], Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
