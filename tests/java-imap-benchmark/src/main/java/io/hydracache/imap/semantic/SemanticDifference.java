package io.hydracache.imap.semantic;

import java.util.Objects;

/** One deterministic mismatch between adapters. */
public record SemanticDifference(String path, String left, String right) {
  public SemanticDifference {
    if (Objects.requireNonNull(path, "path").isBlank()) {
      throw new IllegalArgumentException("path must not be blank");
    }
    Objects.requireNonNull(left, "left");
    Objects.requireNonNull(right, "right");
  }
}
