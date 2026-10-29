"""Separate final doc/byte receipts from immutable original server controls."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-stateful-074/retain_verification.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='d42057672799fd4913ca3d386ddc22c915ec795d187336048279ace05df72265'
text=raw.decode()
for before,after in (
    ('chitchat-stateful-verification-20261029','chitchat-host-cost-verification-20261029'),
    ('chitchat-stateful-result-20261029.md','chitchat-host-cost-result-20261029.md'),
    ('hc074-chitchat-stateful-verification-v1','hc074-chitchat-host-cost-verification-v1'),
    ('0b195601989e9109725812ef5f8da76b3b2c9b4b42aed57e8e7992b6a25f35fa','134607dd8e6f8dced2bb1f0f1eb1b594f1ccc70c802a8c7d601bd64b2cf7e670')):
    assert text.count(before)==1;text=text.replace(before,after)
text=text.replace("for name in DOCS:add", "DOCS=(*DOCS,'docs/testing/performance/0.74/chitchat-cost-exclusive-host-plan-20261029.md')\nfor name in DOCS:add")
exec(compile(text,__file__+'[native-control-verification]','exec'),globals())
