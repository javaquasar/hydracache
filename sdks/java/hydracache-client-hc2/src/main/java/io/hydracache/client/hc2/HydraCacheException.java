package io.hydracache.client.hc2;

import java.util.Objects;

/** Stable HC/2 failure. Safe detail is supplied by the peer or local policy. */
public final class HydraCacheException extends RuntimeException {
  private final ErrorCode code;
  private final RetryAdvice retryAdvice;

  public HydraCacheException(ErrorCode code, RetryAdvice retryAdvice, String safeDetail) {
    super(Objects.requireNonNull(safeDetail, "safeDetail"));
    this.code = Objects.requireNonNull(code, "code");
    this.retryAdvice = Objects.requireNonNull(retryAdvice, "retryAdvice");
  }

  public HydraCacheException(
      ErrorCode code, RetryAdvice retryAdvice, String safeDetail, Throwable cause) {
    super(Objects.requireNonNull(safeDetail, "safeDetail"), cause);
    this.code = Objects.requireNonNull(code, "code");
    this.retryAdvice = Objects.requireNonNull(retryAdvice, "retryAdvice");
  }

  public ErrorCode code() {
    return code;
  }

  public RetryAdvice retryAdvice() {
    return retryAdvice;
  }
}
