package io.hydracache.imap.semantic;

import java.util.Objects;

/** One key/value input in a point or bulk mutation. */
public record ScenarioEntry(BytesValue key, BytesValue value) {
  public ScenarioEntry {
    Objects.requireNonNull(key, "key");
    Objects.requireNonNull(value, "value");
  }
}
