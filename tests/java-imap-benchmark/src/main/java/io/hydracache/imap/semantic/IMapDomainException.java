package io.hydracache.imap.semantic;

/** Typed local-domain failure; no transport code or protocol identity is assigned here. */
public final class IMapDomainException extends RuntimeException {
  public enum Kind {
    INVALID_ARGUMENT("invalid_argument"), KEY_TOO_LARGE("key_too_large"),
    VALUE_TOO_LARGE("value_too_large"), REQUEST_TOO_LARGE("request_too_large"),
    RESPONSE_TOO_LARGE("response_too_large"), CAPACITY("capacity"),
    MUTATION_IDENTITY_CONFLICT("mutation_identity_conflict"), STALE_CURSOR("scan_cursor_stale"),
    CURSOR_MISMATCH("cursor_mismatch"), MAP_DESTROYED("map_destroyed"),
    VERSION_EXHAUSTED("capacity"), UNSUPPORTED_OPERATION("unsupported_operation"),
    IDEMPOTENCY_CAPACITY_EXHAUSTED("idempotency_capacity_exhausted"), CLOCK_REGRESSED("clock_regressed"),
    UNKNOWN_LISTENER("unknown_listener");
    private final String id;
    Kind(String id) { this.id = id; }
    public String id() { return id; }
  }

  private final Kind error;
  public IMapDomainException(Kind error, String detail) {
    super(detail);
    this.error = java.util.Objects.requireNonNull(error, "error");
  }
  public Kind error() { return error; }
  public String kind() { return error.id(); }

  static IMapDomainException translate(RuntimeException failure) {
    if (failure instanceof IMapDomainException domain) return domain;
    String detail = failure.getMessage() == null ? "invalid map request" : failure.getMessage();
    Kind kind = Kind.INVALID_ARGUMENT;
    if (failure instanceof ArithmeticException) kind = Kind.VERSION_EXHAUSTED;
    else if (detail.contains("mutation conflict")) kind = Kind.MUTATION_IDENTITY_CONFLICT;
    else if (detail.contains("keyBytes")) kind = Kind.KEY_TOO_LARGE;
    else if (detail.contains("valueBytes")) kind = Kind.VALUE_TOO_LARGE;
    else if (detail.contains("responseBytes") || detail.contains("response byte")) kind = Kind.RESPONSE_TOO_LARGE;
    else if (detail.contains("requestEntries") || detail.contains("request byte") || detail.contains("bulkMutation")) kind = Kind.REQUEST_TOO_LARGE;
    else if (detail.contains("dedupEntries")) kind = Kind.IDEMPOTENCY_CAPACITY_EXHAUSTED;
    else if (detail.contains("logical time regressed")) kind = Kind.CLOCK_REGRESSED;
    else if (detail.contains("entries exceeds") || detail.contains("capacity")) kind = Kind.CAPACITY;
    else if (detail.contains("stale")) kind = Kind.STALE_CURSOR;
    else if (detail.contains("foreign") || detail.contains("invalid cursor")) kind = Kind.CURSOR_MISMATCH;
    else if (detail.contains("destroyed")) kind = Kind.MAP_DESTROYED;
    return new IMapDomainException(kind, detail);
  }
}
