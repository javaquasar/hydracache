from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum
from pathlib import Path
from typing import Optional


class Capability(IntEnum):
    DATA = 1
    BATCH = 2
    SUBSCRIPTIONS = 3
    TOPOLOGY = 4
    FENCED_SESSIONS = 5
    DIAGNOSTICS = 6


@dataclass(frozen=True)
class ClientLimits:
    max_inbound_message_bytes: int = 4 * 1024 * 1024
    max_outbound_frames: int = 1024
    max_pending_invocations: int = 4096
    max_subscriptions: int = 1024
    max_subscription_events: int = 1024
    max_sessions: int = 1024

    def validate(self) -> None:
        bounds = (
            (self.max_inbound_message_bytes, 1024, 16 * 1024 * 1024),
            (self.max_outbound_frames, 1, 65_536),
            (self.max_pending_invocations, 1, 1_000_000),
            (self.max_subscriptions, 1, 65_536),
            (self.max_subscription_events, 1, 65_536),
            (self.max_sessions, 1, 65_536),
        )
        if any(value < minimum or value > maximum for value, minimum, maximum in bounds):
            raise ValueError("client limit is outside its supported range")


@dataclass(frozen=True)
class ClientConfig:
    endpoint: str
    client_id: str
    tenant: str
    root_certificate: Optional[str | Path] = None
    client_certificate: Optional[str | Path] = None
    client_private_key: Optional[str | Path] = None
    server_name: Optional[str] = None
    insecure: bool = False
    protocol_generation: int = 6
    connect_timeout: float = 10.0
    request_timeout: float = 5.0
    reconnect_max_attempts: int = 5
    reconnect_backoff: float = 0.1
    capabilities: tuple[Capability, ...] = field(default_factory=lambda: tuple(Capability))
    limits: ClientLimits = field(default_factory=ClientLimits)

    def validate(self) -> None:
        if not _safe_text(self.endpoint, 512) or ":" not in self.endpoint:
            raise ValueError("endpoint must be a bounded host:port authority")
        if not _safe_text(self.client_id, 128) or not _safe_text(self.tenant, 128):
            raise ValueError("client identity is blank, oversized, or contains controls")
        if self.protocol_generation not in (5, 6):
            raise ValueError("unsupported HC/2 protocol generation")
        if not 0 < self.connect_timeout <= 300 or not 0 < self.request_timeout <= 300:
            raise ValueError("timeouts must be positive and at most five minutes")
        if not 0 <= self.reconnect_max_attempts <= 100:
            raise ValueError("reconnect_max_attempts must be in [0, 100]")
        if not 0 <= self.reconnect_backoff <= 30:
            raise ValueError("reconnect_backoff must be in [0, 30]")
        if not self.capabilities or len(set(self.capabilities)) != len(self.capabilities):
            raise ValueError("capabilities must be non-empty and unique")
        self.limits.validate()
        tls_files = (self.root_certificate, self.client_certificate, self.client_private_key)
        if self.insecure:
            if any(tls_files):
                raise ValueError("insecure mode cannot also configure TLS credentials")
        elif not all(tls_files) or not _safe_text(self.server_name or "", 253):
            raise ValueError("mTLS requires CA, client certificate, private key, and server_name")


@dataclass(frozen=True)
class RequestOptions:
    timeout: Optional[float] = None
    idempotency_key: bytes = b""

    def validate(self, default_timeout: float) -> float:
        timeout = default_timeout if self.timeout is None else self.timeout
        if not 0 < timeout <= 300:
            raise ValueError("request timeout must be positive and at most five minutes")
        if len(self.idempotency_key) > 128:
            raise ValueError("idempotency key exceeds 128 bytes")
        return timeout


@dataclass(frozen=True)
class CacheValue:
    value: bytes
    expires_at_unix_ms: Optional[int]


@dataclass(frozen=True)
class MutationResult:
    applied: bool


@dataclass(frozen=True)
class CacheEvent:
    subscription_id: int
    watermark: int
    key: bytes
    value: bytes
    removed: bool


@dataclass(frozen=True)
class EventGap:
    subscription_id: int
    after_watermark: int


@dataclass(frozen=True)
class ClientMetrics:
    submitted: int
    completed: int
    failed: int
    cancelled: int
    reconnects: int
    pending_invocations: int
    active_subscriptions: int
    active_sessions: int


def _safe_text(value: str, maximum: int) -> bool:
    return bool(value) and len(value) <= maximum and not any(ord(ch) < 32 or ord(ch) == 127 for ch in value)
