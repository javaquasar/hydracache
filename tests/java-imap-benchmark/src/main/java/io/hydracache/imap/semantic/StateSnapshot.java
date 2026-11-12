package io.hydracache.imap.semantic;

import java.util.Objects;

/** Final state evidence used by the pre-performance semantic gate. */
public record StateSnapshot(String digest, int liveCardinality, long logicalTick) {
  public StateSnapshot {
    if (!Objects.requireNonNull(digest, "digest").matches("[0-9a-f]{64}")) {
      throw new IllegalArgumentException("digest must be lowercase SHA-256");
    }
    if (liveCardinality < 0 || logicalTick < 0) {
      throw new IllegalArgumentException("snapshot counts must be non-negative");
    }
  }
}
