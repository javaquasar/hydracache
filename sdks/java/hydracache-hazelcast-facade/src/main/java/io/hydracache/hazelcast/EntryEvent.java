package io.hydracache.hazelcast;

import java.util.Optional;

/** Cache-signal event. It is not a durable business event. */
public record EntryEvent<K, V>(K key, Optional<V> value, boolean removed, long watermark) {
  public EntryEvent {
    if (key == null) throw new NullPointerException("key");
    value = value == null ? Optional.empty() : value;
    if (watermark < 0) throw new IllegalArgumentException("watermark must be nonnegative");
  }
}
