package io.hydracache.client.hc2;

import java.util.Objects;
import java.util.Optional;

/** Outcome and optional applied version proved by a distributed owner. */
public record BatchItemReceipt(int partitionId, State state, Optional<AppliedVersion> applied) {
  public enum State { CONFIRMED, NOT_APPLIED, OUTCOME_UNKNOWN }

  public record AppliedVersion(long epoch, long version) {
    public AppliedVersion {
      if (epoch <= 0 || version < 0) throw new IllegalArgumentException("invalid applied version");
    }
  }

  public BatchItemReceipt {
    Objects.requireNonNull(state, "state");
    Objects.requireNonNull(applied, "applied");
    if (partitionId < 0 || (state != State.CONFIRMED && applied.isPresent())) {
      throw new IllegalArgumentException("invalid batch receipt");
    }
  }
}
