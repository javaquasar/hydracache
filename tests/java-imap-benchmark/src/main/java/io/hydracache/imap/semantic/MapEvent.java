package io.hydracache.imap.semantic;

import java.util.Objects;
import java.util.Optional;

/** Authoritative transition projected by a semantic adapter. */
public record MapEvent(Kind kind, BytesValue key, Optional<BytesValue> value, long logicalTick) {
  public enum Kind { ADDED, UPDATED, REMOVED, EXPIRED, INVALIDATED, GAP }

  public MapEvent {
    Objects.requireNonNull(kind, "kind");
    Objects.requireNonNull(key, "key");
    value = Objects.requireNonNull(value, "value");
    if (logicalTick < 0) throw new IllegalArgumentException("logicalTick must be non-negative");
  }
}
