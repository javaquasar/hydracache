package io.hydracache.client.hc2;

/** Stable protocol errors exposed without generated wire types. */
public enum ErrorCode {
  INVALID_REQUEST,
  UNAUTHENTICATED,
  UNAUTHORIZED,
  NOT_FOUND,
  CONFLICT,
  QUOTA_EXCEEDED,
  DEADLINE_EXCEEDED,
  UNAVAILABLE,
  UNSUPPORTED,
  GAP_REPAIR_REQUIRED,
  SESSION_LOST,
  INTERNAL,
  UNKNOWN
}
