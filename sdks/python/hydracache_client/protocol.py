"""Protocol-v1 SDK helpers shared by the Python conformance runner."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

PROTOCOL_VERSION = 1


class StableErrorCode(str, Enum):
    INCOMPATIBLE_VERSION = "incompatible_version"
    UNAUTHENTICATED = "unauthenticated"
    UNAUTHORIZED = "unauthorized"
    TENANT_QUOTA = "tenant_quota"
    RATE_LIMITED = "rate_limited"
    RESIDENCY_DENIED = "residency_denied"
    TOO_LARGE = "too_large"
    DEADLINE_EXCEEDED = "deadline_exceeded"
    CONFLICT = "conflict"
    BACKEND_UNAVAILABLE = "backend_unavailable"
    MALFORMED_FRAME = "malformed_frame"


def stable_error_retryable(code: StableErrorCode) -> bool:
    return code in {
        StableErrorCode.TENANT_QUOTA,
        StableErrorCode.RATE_LIMITED,
        StableErrorCode.DEADLINE_EXCEEDED,
        StableErrorCode.BACKEND_UNAVAILABLE,
    }


class RepairAction(str, Enum):
    APPLY = "apply"
    CLEAR_PARTITION = "clear_partition"
    INVALIDATE_CONSERVATIVELY = "invalidate_conservatively"


@dataclass(frozen=True)
class Watermark:
    source_generation: int
    message_id: int


class SubscriptionWatermarkTracker:
    def __init__(self) -> None:
        self._last: Watermark | None = None

    def on_watermark(self, generation: int, message_id: int) -> RepairAction:
        next_watermark = Watermark(generation, message_id)
        if self._last is None:
            self._last = next_watermark
            return RepairAction.CLEAR_PARTITION

        last = self._last
        self._last = next_watermark
        if next_watermark.source_generation != last.source_generation:
            return RepairAction.CLEAR_PARTITION
        if next_watermark.message_id > last.message_id + 1:
            return RepairAction.INVALIDATE_CONSERVATIVELY
        return RepairAction.APPLY
