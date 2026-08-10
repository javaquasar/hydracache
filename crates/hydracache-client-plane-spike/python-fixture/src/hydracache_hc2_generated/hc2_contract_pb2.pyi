from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Capability(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CAPABILITY_UNSPECIFIED: _ClassVar[Capability]
    CAPABILITY_DATA: _ClassVar[Capability]
    CAPABILITY_BATCH: _ClassVar[Capability]
    CAPABILITY_SUBSCRIPTIONS: _ClassVar[Capability]
    CAPABILITY_TOPOLOGY: _ClassVar[Capability]
    CAPABILITY_FENCED_SESSIONS: _ClassVar[Capability]
    CAPABILITY_DIAGNOSTICS: _ClassVar[Capability]

class StableErrorCode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STABLE_ERROR_UNSPECIFIED: _ClassVar[StableErrorCode]
    STABLE_ERROR_INVALID_REQUEST: _ClassVar[StableErrorCode]
    STABLE_ERROR_UNAUTHENTICATED: _ClassVar[StableErrorCode]
    STABLE_ERROR_UNAUTHORIZED: _ClassVar[StableErrorCode]
    STABLE_ERROR_NOT_FOUND: _ClassVar[StableErrorCode]
    STABLE_ERROR_CONFLICT: _ClassVar[StableErrorCode]
    STABLE_ERROR_QUOTA_EXCEEDED: _ClassVar[StableErrorCode]
    STABLE_ERROR_DEADLINE_EXCEEDED: _ClassVar[StableErrorCode]
    STABLE_ERROR_UNAVAILABLE: _ClassVar[StableErrorCode]
    STABLE_ERROR_UNSUPPORTED: _ClassVar[StableErrorCode]
    STABLE_ERROR_GAP_REPAIR_REQUIRED: _ClassVar[StableErrorCode]
    STABLE_ERROR_SESSION_LOST: _ClassVar[StableErrorCode]
    STABLE_ERROR_INTERNAL: _ClassVar[StableErrorCode]

class RetryDirective(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RETRY_DIRECTIVE_UNSPECIFIED: _ClassVar[RetryDirective]
    RETRY_DIRECTIVE_NEVER: _ClassVar[RetryDirective]
    RETRY_DIRECTIVE_SAME_CONNECTION: _ClassVar[RetryDirective]
    RETRY_DIRECTIVE_RECONNECT_IDEMPOTENT: _ClassVar[RetryDirective]
    RETRY_DIRECTIVE_REPAIR_REQUIRED: _ClassVar[RetryDirective]
CAPABILITY_UNSPECIFIED: Capability
CAPABILITY_DATA: Capability
CAPABILITY_BATCH: Capability
CAPABILITY_SUBSCRIPTIONS: Capability
CAPABILITY_TOPOLOGY: Capability
CAPABILITY_FENCED_SESSIONS: Capability
CAPABILITY_DIAGNOSTICS: Capability
STABLE_ERROR_UNSPECIFIED: StableErrorCode
STABLE_ERROR_INVALID_REQUEST: StableErrorCode
STABLE_ERROR_UNAUTHENTICATED: StableErrorCode
STABLE_ERROR_UNAUTHORIZED: StableErrorCode
STABLE_ERROR_NOT_FOUND: StableErrorCode
STABLE_ERROR_CONFLICT: StableErrorCode
STABLE_ERROR_QUOTA_EXCEEDED: StableErrorCode
STABLE_ERROR_DEADLINE_EXCEEDED: StableErrorCode
STABLE_ERROR_UNAVAILABLE: StableErrorCode
STABLE_ERROR_UNSUPPORTED: StableErrorCode
STABLE_ERROR_GAP_REPAIR_REQUIRED: StableErrorCode
STABLE_ERROR_SESSION_LOST: StableErrorCode
STABLE_ERROR_INTERNAL: StableErrorCode
RETRY_DIRECTIVE_UNSPECIFIED: RetryDirective
RETRY_DIRECTIVE_NEVER: RetryDirective
RETRY_DIRECTIVE_SAME_CONNECTION: RetryDirective
RETRY_DIRECTIVE_RECONNECT_IDEMPOTENT: RetryDirective
RETRY_DIRECTIVE_REPAIR_REQUIRED: RetryDirective

class RequestMeta(_message.Message):
    __slots__ = ("deadline_unix_ms", "idempotency_key", "tenant", "topology_epoch")
    DEADLINE_UNIX_MS_FIELD_NUMBER: _ClassVar[int]
    IDEMPOTENCY_KEY_FIELD_NUMBER: _ClassVar[int]
    TENANT_FIELD_NUMBER: _ClassVar[int]
    TOPOLOGY_EPOCH_FIELD_NUMBER: _ClassVar[int]
    deadline_unix_ms: int
    idempotency_key: bytes
    tenant: str
    topology_epoch: int
    def __init__(self, deadline_unix_ms: _Optional[int] = ..., idempotency_key: _Optional[bytes] = ..., tenant: _Optional[str] = ..., topology_epoch: _Optional[int] = ...) -> None: ...

class ResponseMeta(_message.Message):
    __slots__ = ("error", "retry", "safe_detail", "topology_epoch")
    ERROR_FIELD_NUMBER: _ClassVar[int]
    RETRY_FIELD_NUMBER: _ClassVar[int]
    SAFE_DETAIL_FIELD_NUMBER: _ClassVar[int]
    TOPOLOGY_EPOCH_FIELD_NUMBER: _ClassVar[int]
    error: StableErrorCode
    retry: RetryDirective
    safe_detail: str
    topology_epoch: int
    def __init__(self, error: _Optional[_Union[StableErrorCode, str]] = ..., retry: _Optional[_Union[RetryDirective, str]] = ..., safe_detail: _Optional[str] = ..., topology_epoch: _Optional[int] = ...) -> None: ...

class Handshake(_message.Message):
    __slots__ = ("generation", "client_id", "requested", "connection_generation")
    GENERATION_FIELD_NUMBER: _ClassVar[int]
    CLIENT_ID_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_GENERATION_FIELD_NUMBER: _ClassVar[int]
    generation: int
    client_id: str
    requested: _containers.RepeatedScalarFieldContainer[Capability]
    connection_generation: int
    def __init__(self, generation: _Optional[int] = ..., client_id: _Optional[str] = ..., requested: _Optional[_Iterable[_Union[Capability, str]]] = ..., connection_generation: _Optional[int] = ...) -> None: ...

class HandshakeAck(_message.Message):
    __slots__ = ("generation", "cluster_id", "accepted", "topology_epoch", "connection_generation")
    GENERATION_FIELD_NUMBER: _ClassVar[int]
    CLUSTER_ID_FIELD_NUMBER: _ClassVar[int]
    ACCEPTED_FIELD_NUMBER: _ClassVar[int]
    TOPOLOGY_EPOCH_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_GENERATION_FIELD_NUMBER: _ClassVar[int]
    generation: int
    cluster_id: str
    accepted: _containers.RepeatedScalarFieldContainer[Capability]
    topology_epoch: int
    connection_generation: int
    def __init__(self, generation: _Optional[int] = ..., cluster_id: _Optional[str] = ..., accepted: _Optional[_Iterable[_Union[Capability, str]]] = ..., topology_epoch: _Optional[int] = ..., connection_generation: _Optional[int] = ...) -> None: ...

class GetRequest(_message.Message):
    __slots__ = ("key",)
    KEY_FIELD_NUMBER: _ClassVar[int]
    key: bytes
    def __init__(self, key: _Optional[bytes] = ...) -> None: ...

class PutRequest(_message.Message):
    __slots__ = ("key", "value", "ttl_ms")
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    TTL_MS_FIELD_NUMBER: _ClassVar[int]
    key: bytes
    value: bytes
    ttl_ms: int
    def __init__(self, key: _Optional[bytes] = ..., value: _Optional[bytes] = ..., ttl_ms: _Optional[int] = ...) -> None: ...

class DeleteRequest(_message.Message):
    __slots__ = ("key",)
    KEY_FIELD_NUMBER: _ClassVar[int]
    key: bytes
    def __init__(self, key: _Optional[bytes] = ...) -> None: ...

class CompareAndSetRequest(_message.Message):
    __slots__ = ("key", "expected", "replacement", "ttl_ms")
    KEY_FIELD_NUMBER: _ClassVar[int]
    EXPECTED_FIELD_NUMBER: _ClassVar[int]
    REPLACEMENT_FIELD_NUMBER: _ClassVar[int]
    TTL_MS_FIELD_NUMBER: _ClassVar[int]
    key: bytes
    expected: bytes
    replacement: bytes
    ttl_ms: int
    def __init__(self, key: _Optional[bytes] = ..., expected: _Optional[bytes] = ..., replacement: _Optional[bytes] = ..., ttl_ms: _Optional[int] = ...) -> None: ...

class BatchItem(_message.Message):
    __slots__ = ("item_id", "get", "put", "delete", "compare_and_set")
    ITEM_ID_FIELD_NUMBER: _ClassVar[int]
    GET_FIELD_NUMBER: _ClassVar[int]
    PUT_FIELD_NUMBER: _ClassVar[int]
    DELETE_FIELD_NUMBER: _ClassVar[int]
    COMPARE_AND_SET_FIELD_NUMBER: _ClassVar[int]
    item_id: int
    get: GetRequest
    put: PutRequest
    delete: DeleteRequest
    compare_and_set: CompareAndSetRequest
    def __init__(self, item_id: _Optional[int] = ..., get: _Optional[_Union[GetRequest, _Mapping]] = ..., put: _Optional[_Union[PutRequest, _Mapping]] = ..., delete: _Optional[_Union[DeleteRequest, _Mapping]] = ..., compare_and_set: _Optional[_Union[CompareAndSetRequest, _Mapping]] = ...) -> None: ...

class BatchRequest(_message.Message):
    __slots__ = ("items",)
    ITEMS_FIELD_NUMBER: _ClassVar[int]
    items: _containers.RepeatedCompositeFieldContainer[BatchItem]
    def __init__(self, items: _Optional[_Iterable[_Union[BatchItem, _Mapping]]] = ...) -> None: ...

class InvocationRequest(_message.Message):
    __slots__ = ("meta", "get", "put", "delete", "compare_and_set", "batch")
    META_FIELD_NUMBER: _ClassVar[int]
    GET_FIELD_NUMBER: _ClassVar[int]
    PUT_FIELD_NUMBER: _ClassVar[int]
    DELETE_FIELD_NUMBER: _ClassVar[int]
    COMPARE_AND_SET_FIELD_NUMBER: _ClassVar[int]
    BATCH_FIELD_NUMBER: _ClassVar[int]
    meta: RequestMeta
    get: GetRequest
    put: PutRequest
    delete: DeleteRequest
    compare_and_set: CompareAndSetRequest
    batch: BatchRequest
    def __init__(self, meta: _Optional[_Union[RequestMeta, _Mapping]] = ..., get: _Optional[_Union[GetRequest, _Mapping]] = ..., put: _Optional[_Union[PutRequest, _Mapping]] = ..., delete: _Optional[_Union[DeleteRequest, _Mapping]] = ..., compare_and_set: _Optional[_Union[CompareAndSetRequest, _Mapping]] = ..., batch: _Optional[_Union[BatchRequest, _Mapping]] = ...) -> None: ...

class ValueResult(_message.Message):
    __slots__ = ("found", "value", "expires_at_unix_ms")
    FOUND_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_UNIX_MS_FIELD_NUMBER: _ClassVar[int]
    found: bool
    value: bytes
    expires_at_unix_ms: int
    def __init__(self, found: bool = ..., value: _Optional[bytes] = ..., expires_at_unix_ms: _Optional[int] = ...) -> None: ...

class MutationResult(_message.Message):
    __slots__ = ("applied",)
    APPLIED_FIELD_NUMBER: _ClassVar[int]
    applied: bool
    def __init__(self, applied: bool = ...) -> None: ...

class BatchResult(_message.Message):
    __slots__ = ("items",)
    ITEMS_FIELD_NUMBER: _ClassVar[int]
    items: _containers.RepeatedCompositeFieldContainer[InvocationResponse]
    def __init__(self, items: _Optional[_Iterable[_Union[InvocationResponse, _Mapping]]] = ...) -> None: ...

class InvocationResponse(_message.Message):
    __slots__ = ("meta", "value", "mutation", "batch")
    META_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    MUTATION_FIELD_NUMBER: _ClassVar[int]
    BATCH_FIELD_NUMBER: _ClassVar[int]
    meta: ResponseMeta
    value: ValueResult
    mutation: MutationResult
    batch: BatchResult
    def __init__(self, meta: _Optional[_Union[ResponseMeta, _Mapping]] = ..., value: _Optional[_Union[ValueResult, _Mapping]] = ..., mutation: _Optional[_Union[MutationResult, _Mapping]] = ..., batch: _Optional[_Union[BatchResult, _Mapping]] = ...) -> None: ...

class Cancel(_message.Message):
    __slots__ = ("correlation_id", "safe_reason")
    CORRELATION_ID_FIELD_NUMBER: _ClassVar[int]
    SAFE_REASON_FIELD_NUMBER: _ClassVar[int]
    correlation_id: int
    safe_reason: str
    def __init__(self, correlation_id: _Optional[int] = ..., safe_reason: _Optional[str] = ...) -> None: ...

class Subscribe(_message.Message):
    __slots__ = ("subscription_id", "key_prefix", "resume_watermark")
    SUBSCRIPTION_ID_FIELD_NUMBER: _ClassVar[int]
    KEY_PREFIX_FIELD_NUMBER: _ClassVar[int]
    RESUME_WATERMARK_FIELD_NUMBER: _ClassVar[int]
    subscription_id: int
    key_prefix: bytes
    resume_watermark: int
    def __init__(self, subscription_id: _Optional[int] = ..., key_prefix: _Optional[bytes] = ..., resume_watermark: _Optional[int] = ...) -> None: ...

class Unsubscribe(_message.Message):
    __slots__ = ("subscription_id",)
    SUBSCRIPTION_ID_FIELD_NUMBER: _ClassVar[int]
    subscription_id: int
    def __init__(self, subscription_id: _Optional[int] = ...) -> None: ...

class SubscriptionAck(_message.Message):
    __slots__ = ("subscription_id", "watermark")
    SUBSCRIPTION_ID_FIELD_NUMBER: _ClassVar[int]
    WATERMARK_FIELD_NUMBER: _ClassVar[int]
    subscription_id: int
    watermark: int
    def __init__(self, subscription_id: _Optional[int] = ..., watermark: _Optional[int] = ...) -> None: ...

class CacheEvent(_message.Message):
    __slots__ = ("subscription_id", "watermark", "key", "value", "removed")
    SUBSCRIPTION_ID_FIELD_NUMBER: _ClassVar[int]
    WATERMARK_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    REMOVED_FIELD_NUMBER: _ClassVar[int]
    subscription_id: int
    watermark: int
    key: bytes
    value: bytes
    removed: bool
    def __init__(self, subscription_id: _Optional[int] = ..., watermark: _Optional[int] = ..., key: _Optional[bytes] = ..., value: _Optional[bytes] = ..., removed: bool = ...) -> None: ...

class EventGap(_message.Message):
    __slots__ = ("subscription_id", "after_watermark")
    SUBSCRIPTION_ID_FIELD_NUMBER: _ClassVar[int]
    AFTER_WATERMARK_FIELD_NUMBER: _ClassVar[int]
    subscription_id: int
    after_watermark: int
    def __init__(self, subscription_id: _Optional[int] = ..., after_watermark: _Optional[int] = ...) -> None: ...

class NodeEndpoint(_message.Message):
    __slots__ = ("node_id", "node_epoch", "endpoint_uri", "server_name")
    NODE_ID_FIELD_NUMBER: _ClassVar[int]
    NODE_EPOCH_FIELD_NUMBER: _ClassVar[int]
    ENDPOINT_URI_FIELD_NUMBER: _ClassVar[int]
    SERVER_NAME_FIELD_NUMBER: _ClassVar[int]
    node_id: str
    node_epoch: int
    endpoint_uri: str
    server_name: str
    def __init__(self, node_id: _Optional[str] = ..., node_epoch: _Optional[int] = ..., endpoint_uri: _Optional[str] = ..., server_name: _Optional[str] = ...) -> None: ...

class TopologyUpdate(_message.Message):
    __slots__ = ("epoch", "nodes")
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    NODES_FIELD_NUMBER: _ClassVar[int]
    epoch: int
    nodes: _containers.RepeatedCompositeFieldContainer[NodeEndpoint]
    def __init__(self, epoch: _Optional[int] = ..., nodes: _Optional[_Iterable[_Union[NodeEndpoint, _Mapping]]] = ...) -> None: ...

class SessionOpen(_message.Message):
    __slots__ = ("requested_ttl_ms",)
    REQUESTED_TTL_MS_FIELD_NUMBER: _ClassVar[int]
    requested_ttl_ms: int
    def __init__(self, requested_ttl_ms: _Optional[int] = ...) -> None: ...

class SessionHeartbeat(_message.Message):
    __slots__ = ("session_id", "fence")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    FENCE_FIELD_NUMBER: _ClassVar[int]
    session_id: bytes
    fence: int
    def __init__(self, session_id: _Optional[bytes] = ..., fence: _Optional[int] = ...) -> None: ...

class SessionClose(_message.Message):
    __slots__ = ("session_id", "fence")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    FENCE_FIELD_NUMBER: _ClassVar[int]
    session_id: bytes
    fence: int
    def __init__(self, session_id: _Optional[bytes] = ..., fence: _Optional[int] = ...) -> None: ...

class SessionLost(_message.Message):
    __slots__ = ("session_id", "last_fence")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    LAST_FENCE_FIELD_NUMBER: _ClassVar[int]
    session_id: bytes
    last_fence: int
    def __init__(self, session_id: _Optional[bytes] = ..., last_fence: _Optional[int] = ...) -> None: ...

class Diagnostics(_message.Message):
    __slots__ = ("pending_invocations", "queued_reply_bytes", "queued_event_bytes", "active_subscriptions", "rejected_frames")
    PENDING_INVOCATIONS_FIELD_NUMBER: _ClassVar[int]
    QUEUED_REPLY_BYTES_FIELD_NUMBER: _ClassVar[int]
    QUEUED_EVENT_BYTES_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_SUBSCRIPTIONS_FIELD_NUMBER: _ClassVar[int]
    REJECTED_FRAMES_FIELD_NUMBER: _ClassVar[int]
    pending_invocations: int
    queued_reply_bytes: int
    queued_event_bytes: int
    active_subscriptions: int
    rejected_frames: int
    def __init__(self, pending_invocations: _Optional[int] = ..., queued_reply_bytes: _Optional[int] = ..., queued_event_bytes: _Optional[int] = ..., active_subscriptions: _Optional[int] = ..., rejected_frames: _Optional[int] = ...) -> None: ...

class ClientEnvelope(_message.Message):
    __slots__ = ("generation", "connection_generation", "correlation_id", "handshake", "invocation", "cancel", "subscribe", "session_open", "session_heartbeat", "unsubscribe", "session_close")
    GENERATION_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_GENERATION_FIELD_NUMBER: _ClassVar[int]
    CORRELATION_ID_FIELD_NUMBER: _ClassVar[int]
    HANDSHAKE_FIELD_NUMBER: _ClassVar[int]
    INVOCATION_FIELD_NUMBER: _ClassVar[int]
    CANCEL_FIELD_NUMBER: _ClassVar[int]
    SUBSCRIBE_FIELD_NUMBER: _ClassVar[int]
    SESSION_OPEN_FIELD_NUMBER: _ClassVar[int]
    SESSION_HEARTBEAT_FIELD_NUMBER: _ClassVar[int]
    UNSUBSCRIBE_FIELD_NUMBER: _ClassVar[int]
    SESSION_CLOSE_FIELD_NUMBER: _ClassVar[int]
    generation: int
    connection_generation: int
    correlation_id: int
    handshake: Handshake
    invocation: InvocationRequest
    cancel: Cancel
    subscribe: Subscribe
    session_open: SessionOpen
    session_heartbeat: SessionHeartbeat
    unsubscribe: Unsubscribe
    session_close: SessionClose
    def __init__(self, generation: _Optional[int] = ..., connection_generation: _Optional[int] = ..., correlation_id: _Optional[int] = ..., handshake: _Optional[_Union[Handshake, _Mapping]] = ..., invocation: _Optional[_Union[InvocationRequest, _Mapping]] = ..., cancel: _Optional[_Union[Cancel, _Mapping]] = ..., subscribe: _Optional[_Union[Subscribe, _Mapping]] = ..., session_open: _Optional[_Union[SessionOpen, _Mapping]] = ..., session_heartbeat: _Optional[_Union[SessionHeartbeat, _Mapping]] = ..., unsubscribe: _Optional[_Union[Unsubscribe, _Mapping]] = ..., session_close: _Optional[_Union[SessionClose, _Mapping]] = ...) -> None: ...

class ServerEnvelope(_message.Message):
    __slots__ = ("generation", "connection_generation", "correlation_id", "handshake", "invocation", "subscribed", "event", "gap", "topology", "session_heartbeat", "session_lost", "diagnostics")
    GENERATION_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_GENERATION_FIELD_NUMBER: _ClassVar[int]
    CORRELATION_ID_FIELD_NUMBER: _ClassVar[int]
    HANDSHAKE_FIELD_NUMBER: _ClassVar[int]
    INVOCATION_FIELD_NUMBER: _ClassVar[int]
    SUBSCRIBED_FIELD_NUMBER: _ClassVar[int]
    EVENT_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    TOPOLOGY_FIELD_NUMBER: _ClassVar[int]
    SESSION_HEARTBEAT_FIELD_NUMBER: _ClassVar[int]
    SESSION_LOST_FIELD_NUMBER: _ClassVar[int]
    DIAGNOSTICS_FIELD_NUMBER: _ClassVar[int]
    generation: int
    connection_generation: int
    correlation_id: int
    handshake: HandshakeAck
    invocation: InvocationResponse
    subscribed: SubscriptionAck
    event: CacheEvent
    gap: EventGap
    topology: TopologyUpdate
    session_heartbeat: SessionHeartbeat
    session_lost: SessionLost
    diagnostics: Diagnostics
    def __init__(self, generation: _Optional[int] = ..., connection_generation: _Optional[int] = ..., correlation_id: _Optional[int] = ..., handshake: _Optional[_Union[HandshakeAck, _Mapping]] = ..., invocation: _Optional[_Union[InvocationResponse, _Mapping]] = ..., subscribed: _Optional[_Union[SubscriptionAck, _Mapping]] = ..., event: _Optional[_Union[CacheEvent, _Mapping]] = ..., gap: _Optional[_Union[EventGap, _Mapping]] = ..., topology: _Optional[_Union[TopologyUpdate, _Mapping]] = ..., session_heartbeat: _Optional[_Union[SessionHeartbeat, _Mapping]] = ..., session_lost: _Optional[_Union[SessionLost, _Mapping]] = ..., diagnostics: _Optional[_Union[Diagnostics, _Mapping]] = ...) -> None: ...
