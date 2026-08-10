package io.hydracache.client.hc2;

/** Stable server retry instruction. The preview SDK never retries implicitly. */
public enum RetryAdvice {
  NEVER,
  SAME_CONNECTION,
  RECONNECT_IDEMPOTENT,
  REPAIR_REQUIRED,
  UNSPECIFIED
}
