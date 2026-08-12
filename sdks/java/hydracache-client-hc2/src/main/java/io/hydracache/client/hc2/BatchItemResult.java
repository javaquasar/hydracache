package io.hydracache.client.hc2;

import java.util.Optional;
import java.util.Objects;

/** One ordered batch result. Both optionals are empty for a successful GET miss. */
public record BatchItemResult(
    int itemId, Optional<CacheValue> value, Optional<MutationResult> mutation) {
  public BatchItemResult {
    if (itemId <= 0) throw new IllegalArgumentException("itemId must be positive");
    value = Objects.requireNonNull(value, "value");
    mutation = Objects.requireNonNull(mutation, "mutation");
    if (value.isPresent() && mutation.isPresent()) {
      throw new IllegalArgumentException("batch result kinds are mutually exclusive");
    }
  }
}
