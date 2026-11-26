package io.hydracache.client.hc2;

import java.util.Objects;
import java.util.Optional;

/** Original batch position and its independent success or failure. */
public record BatchItemOutcome(
    int itemId, long topologyEpoch, Optional<BatchItemResult> result,
    Optional<HydraCacheException> error) {
  public BatchItemOutcome {
    if (itemId <= 0 || topologyEpoch < 0) {
      throw new IllegalArgumentException("invalid batch item identity");
    }
    Objects.requireNonNull(result, "result");
    Objects.requireNonNull(error, "error");
    if (result.isPresent() == error.isPresent()) {
      throw new IllegalArgumentException("batch item needs exactly one outcome");
    }
  }
}
