"""Fresh output placement only; elevation is requested separately by the operator."""
import hashlib
from pathlib import Path
import sys

import root_namespace_logged as core

JOURNAL = core.ROOT / 'target/wsl-clock-root-namespace-uac-20261023'
CORE_SHA = '9a2dd2701bbbe718b63a3b5989c0f23c786e213a27f2d33a07c8657484e5829c'


def execute(journal):
    if hashlib.sha256(Path(core.__file__).read_bytes()).hexdigest() != CORE_SHA:
        raise ValueError('Journal-first core changed; preserve refusal without dispatch')
    old = core.JOURNAL
    try:
        core.JOURNAL = JOURNAL
        return core.execute(journal)
    finally:
        core.JOURNAL = old


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('Use only the separately approved UAC launcher')
    raise SystemExit(execute(Path(sys.argv[1])))
