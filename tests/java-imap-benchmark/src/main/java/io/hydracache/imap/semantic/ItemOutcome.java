package io.hydracache.imap.semantic;

import java.util.Objects;
import java.util.Optional;

/** One request-ordered bulk outcome. */
public record ItemOutcome(
    int inputIndex, BytesValue key, OutcomeKind kind, Optional<BytesValue> value, TtlState ttl) {
  public ItemOutcome {
    if (inputIndex < 0) throw new IllegalArgumentException("inputIndex must be non-negative");
    Objects.requireNonNull(key, "key");
    Objects.requireNonNull(kind, "kind");
    value = Objects.requireNonNull(value, "value");
    Objects.requireNonNull(ttl, "ttl");
  }
}
