package io.hydracache.client.hc2;

import java.time.Duration;
import java.util.Arrays;
import java.util.Objects;

/** Per-invocation deadline and idempotency policy. */
public record RequestOptions(Duration timeout, byte[] idempotencyKey) {
  private static final Duration MAX_TIMEOUT = Duration.ofMinutes(5);

  public RequestOptions {
    Objects.requireNonNull(timeout, "timeout");
    if (timeout.isNegative() || timeout.isZero()) {
      throw new IllegalArgumentException("timeout must be positive");
    }
    if (timeout.compareTo(MAX_TIMEOUT) > 0) {
      throw new IllegalArgumentException("timeout must not exceed five minutes");
    }
    idempotencyKey = idempotencyKey == null ? new byte[0] : idempotencyKey.clone();
    if (idempotencyKey.length > 128) {
      throw new IllegalArgumentException("idempotency key exceeds 128 bytes");
    }
  }

  public static RequestOptions withTimeout(Duration timeout) {
    return new RequestOptions(timeout, new byte[0]);
  }

  @Override
  public byte[] idempotencyKey() {
    return idempotencyKey.clone();
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof RequestOptions that
        && timeout.equals(that.timeout)
        && Arrays.equals(idempotencyKey, that.idempotencyKey);
  }

  @Override
  public int hashCode() {
    return 31 * timeout.hashCode() + Arrays.hashCode(idempotencyKey);
  }
}
