package io.hydracache.imap.semantic;

import java.util.Objects;

/** Logical-time TTL directive; the semantic preflight never compares wall clocks. */
public record TtlDirective(Kind kind, long ticks) {
  public enum Kind { PRESERVE, ETERNAL, EXPIRE_AFTER }

  public TtlDirective {
    Objects.requireNonNull(kind, "kind");
    if (kind == Kind.EXPIRE_AFTER && ticks <= 0) {
      throw new IllegalArgumentException("expire-after ticks must be positive");
    }
    if (kind != Kind.EXPIRE_AFTER && ticks != 0) {
      throw new IllegalArgumentException("only expire-after carries ticks");
    }
  }

  public static TtlDirective preserve() {
    return new TtlDirective(Kind.PRESERVE, 0);
  }

  public static TtlDirective eternal() {
    return new TtlDirective(Kind.ETERNAL, 0);
  }

  public static TtlDirective expireAfter(long ticks) {
    return new TtlDirective(Kind.EXPIRE_AFTER, ticks);
  }
}
