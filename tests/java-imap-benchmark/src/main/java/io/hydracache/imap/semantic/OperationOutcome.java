package io.hydracache.imap.semantic;

import java.util.List;
import java.util.Objects;
import java.util.Optional;

/** Canonical adapter result compared by the deterministic oracle. */
public record OperationOutcome(
    OutcomeKind kind,
    Optional<BytesValue> value,
    TtlState ttl,
    List<ItemOutcome> items,
    ErrorClass errorClass,
    String detail) {
  public OperationOutcome {
    Objects.requireNonNull(kind, "kind");
    value = Objects.requireNonNull(value, "value");
    Objects.requireNonNull(ttl, "ttl");
    items = List.copyOf(items);
    Objects.requireNonNull(errorClass, "errorClass");
    detail = Objects.requireNonNull(detail, "detail");
    if (kind != OutcomeKind.ERROR && errorClass != ErrorClass.NONE) {
      throw new IllegalArgumentException("non-error result cannot carry error class");
    }
  }

  public static OperationOutcome point(
      OutcomeKind kind, Optional<BytesValue> value, TtlState ttl) {
    return new OperationOutcome(kind, value, ttl, List.of(), ErrorClass.NONE, "");
  }

  public static OperationOutcome bulk(List<ItemOutcome> items) {
    return new OperationOutcome(
        OutcomeKind.BULK, Optional.empty(), TtlState.absent(), items, ErrorClass.NONE, "");
  }

  public static OperationOutcome advanced() {
    return point(OutcomeKind.ADVANCED, Optional.empty(), TtlState.absent());
  }

  public static OperationOutcome error(ErrorClass errorClass, String detail) {
    return new OperationOutcome(OutcomeKind.ERROR, Optional.empty(), TtlState.absent(), List.of(),
        errorClass, detail);
  }
}
