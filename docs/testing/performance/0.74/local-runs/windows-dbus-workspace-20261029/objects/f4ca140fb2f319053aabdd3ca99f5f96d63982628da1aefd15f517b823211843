"""Reconstruct the terminal GET-owner test-source delta from fixed commits only."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    'fixed_source_edits', HERE.parent / 'source-successor-view-074/generate.py')
producer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(producer)
producer.CASES = [('crates/xtask/tests/performance_contract_074.rs',
                   '37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a',
                   'a67a5299ab335424e9b11330dc06bd749ea9b859')]

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(producer.description(), ensure_ascii=False, indent=2) + '\n').encode()
    destination = HERE / 'main-rules.json'
    if args.write:
        with destination.open('xb') as output:
            output.write(data)
    else:
        assert destination.read_bytes() == data, 'fixed terminal test delta differs'
    print(hashlib.sha256(data).hexdigest())
