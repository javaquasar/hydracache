package io.hydracache.imap.semantic;

/** Logical TTL observation; remaining ticks are used only for expiring values. */
public record TtlState(Kind kind, long remainingTicks) {
  public enum Kind { ABSENT, ETERNAL, EXPIRING }

  public TtlState {
    if (kind == null) throw new NullPointerException("kind");
    if (kind == Kind.EXPIRING && remainingTicks <= 0) {
      throw new IllegalArgumentException("expiring TTL must have positive remaining ticks");
    }
    if (kind != Kind.EXPIRING && remainingTicks != 0) {
      throw new IllegalArgumentException("only expiring TTL has remaining ticks");
    }
  }

  public static TtlState absent() { return new TtlState(Kind.ABSENT, 0); }
  public static TtlState eternal() { return new TtlState(Kind.ETERNAL, 0); }
  public static TtlState expiring(long ticks) { return new TtlState(Kind.EXPIRING, ticks); }
}
