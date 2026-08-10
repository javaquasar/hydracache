package io.hydracache.client.hc2;

import java.time.Instant;
import java.util.Optional;
import java.util.Objects;

/** Immutable value returned by HC/2. */
public record CacheValue(byte[] value, Optional<Instant> expiresAt) {
  public CacheValue {
    value = Objects.requireNonNull(value, "value").clone();
    expiresAt = expiresAt == null ? Optional.empty() : expiresAt;
  }

  @Override
  public byte[] value() {
    return value.clone();
  }
}
