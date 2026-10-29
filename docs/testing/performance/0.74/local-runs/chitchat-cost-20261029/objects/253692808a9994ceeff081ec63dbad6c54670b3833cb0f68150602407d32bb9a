"""Pin the unchanged final-receipt writer; use caller documents/output/index."""
from pathlib import Path
import hashlib

old=Path(__file__).resolve().parents[1]/'chitchat-stateful-074/retain_verification.py'
raw=old.read_bytes();assert hashlib.sha256(raw).hexdigest()=='d42057672799fd4913ca3d386ddc22c915ec795d187336048279ace05df72265'
text=raw.decode()
for before,after in (
    ('chitchat-stateful-verification-20261029','chitchat-caller-verification-20261029'),
    ('chitchat-stateful-result-20261029.md','chitchat-caller-result-20261029.md'),
    ('hc074-chitchat-stateful-verification-v1','hc074-chitchat-caller-verification-v1'),
    ('0b195601989e9109725812ef5f8da76b3b2c9b4b42aed57e8e7992b6a25f35fa','874ba6355b98229cbeab11c5b0742e439cbced847e6954478234e10e92d7ef56')):
    assert text.count(before)==1,before;text=text.replace(before,after)
exec(compile(text,__file__+'[receipt-derivative]','exec'),globals())
