from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional

DESCRIPTOR: _descriptor.FileDescriptor

class SpikeEnvelope(_message.Message):
    __slots__ = ("generation", "kind", "correlation_id", "payload")
    GENERATION_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    CORRELATION_ID_FIELD_NUMBER: _ClassVar[int]
    PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    generation: int
    kind: int
    correlation_id: int
    payload: bytes
    def __init__(self, generation: _Optional[int] = ..., kind: _Optional[int] = ..., correlation_id: _Optional[int] = ..., payload: _Optional[bytes] = ...) -> None: ...
