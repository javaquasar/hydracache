"""Recover inert records from preserved bytes; never start or retry a probe."""
import json


def recover(case_id, native_record, stdout, stderr):
    if not isinstance(case_id, str) or not case_id.isascii():
        raise ValueError('original ASCII case identity')
    # The original Windows writer emitted these fields before encoding value.
    # Do not infer successful process termination from stdout alone.
    prefix = (json.dumps(dict(id=case_id, exit=0, timeout=False), indent=2)[:-2]
              + ',\n  "value": ').replace('\n', '\r\n').encode('ascii')
    if not native_record.startswith(prefix) or stderr:
        raise ValueError('recorded original exit/timeout/prefix or stderr')
    value = json.loads(stdout.decode('utf-8'))
    if not isinstance(value, dict) or value.get('id') != case_id:
        raise ValueError('raw response identity')
    return dict(id=case_id, exit=0, timeout=False, value=value)


def write_record(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write('\n')
