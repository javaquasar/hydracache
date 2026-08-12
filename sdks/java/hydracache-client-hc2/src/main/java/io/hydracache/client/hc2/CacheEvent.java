package io.hydracache.client.hc2;

import java.util.Objects;

/** Ordered subscription event. */
public record CacheEvent(long subscriptionId, long watermark, byte[] key, byte[] value, boolean removed) {
  public CacheEvent {
    if (subscriptionId <= 0) throw new IllegalArgumentException("subscriptionId must be positive");
    if (watermark < 0) throw new IllegalArgumentException("watermark must be nonnegative");
    key = Objects.requireNonNull(key, "key").clone();
    value = Objects.requireNonNull(value, "value").clone();
  }
  @Override public byte[] key() { return key.clone(); }
  @Override public byte[] value() { return value.clone(); }
}
