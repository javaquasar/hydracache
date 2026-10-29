"""Retain final document and byte-only receipts separately from first outcomes."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-stateful-074/retain_verification.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='d42057672799fd4913ca3d386ddc22c915ec795d187336048279ace05df72265'
text=raw.decode()
for before,after in (
    ('chitchat-stateful-verification-20261029','chitchat-cost-verification-20261029'),
    ('chitchat-stateful-result-20261029.md','chitchat-cost-result-20261029.md'),
    ('hc074-chitchat-stateful-verification-v1','hc074-chitchat-cost-verification-v1'),
    ('0b195601989e9109725812ef5f8da76b3b2c9b4b42aed57e8e7992b6a25f35fa','9589ebd9014f7b2be15aa8cfedec635e06d042a4b81fe92da3894fdea1008462')):
    assert text.count(before)==1;text=text.replace(before,after)
exec(compile(text,__file__+'[first-refusal-receipt]','exec'),globals())
