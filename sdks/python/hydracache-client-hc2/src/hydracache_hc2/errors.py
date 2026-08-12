from __future__ import annotations

from enum import Enum


class ErrorCode(Enum):
    INVALID_REQUEST = "invalid_request"
    UNAUTHENTICATED = "unauthenticated"
    UNAUTHORIZED = "unauthorized"
    NOT_FOUND = "not_found"
    CONFLICT = "conflict"
    QUOTA_EXCEEDED = "quota_exceeded"
    DEADLINE_EXCEEDED = "deadline_exceeded"
    UNAVAILABLE = "unavailable"
    UNSUPPORTED = "unsupported"
    GAP_REPAIR_REQUIRED = "gap_repair_required"
    SESSION_LOST = "session_lost"
    INTERNAL = "internal"


class RetryAdvice(Enum):
    NEVER = "never"
    SAME_CONNECTION = "same_connection"
    RECONNECT_IDEMPOTENT = "reconnect_idempotent"
    REPAIR_REQUIRED = "repair_required"


class HydraCacheError(Exception):
    """Privacy-safe HC/2 failure with stable retry classification."""

    def __init__(self, code: ErrorCode, retry: RetryAdvice, safe_detail: str):
        super().__init__(f"HC/2 {code.value}: {safe_detail}")
        self.code = code
        self.retry_advice = retry
        self.safe_detail = safe_detail
