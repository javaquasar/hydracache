"""Public asyncio API for the generated HydraCache HC/2 client plane."""

from .client import AsyncHydraCacheClient, FencedSession, Subscription
from .errors import ErrorCode, HydraCacheError, RetryAdvice
from .models import (
    CacheEvent,
    CacheValue,
    Capability,
    ClientConfig,
    ClientLimits,
    ClientMetrics,
    EventGap,
    LockAcquireResult,
    LockOwnership,
    MutationResult,
    RequestOptions,
)

__all__ = [
    "AsyncHydraCacheClient",
    "CacheEvent",
    "CacheValue",
    "Capability",
    "ClientConfig",
    "ClientLimits",
    "ClientMetrics",
    "ErrorCode",
    "EventGap",
    "FencedSession",
    "HydraCacheError",
    "LockAcquireResult",
    "LockOwnership",
    "MutationResult",
    "RequestOptions",
    "RetryAdvice",
    "Subscription",
]

__version__ = "0.68.0a1"
