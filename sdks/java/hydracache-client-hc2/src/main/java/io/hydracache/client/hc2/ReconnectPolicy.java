package io.hydracache.client.hc2;

import java.time.Duration;
import java.util.Objects;

/** Bounded deterministic connection-establishment policy; it never authorizes operation replay. */
public record ReconnectPolicy(
    int maxAttempts, Duration initialBackoff, Duration maxBackoff, Duration jitter, long seed) {
  public ReconnectPolicy {
    Objects.requireNonNull(initialBackoff, "initialBackoff");
    Objects.requireNonNull(maxBackoff, "maxBackoff");
    Objects.requireNonNull(jitter, "jitter");
    if (maxAttempts < 1 || maxAttempts > 32 || initialBackoff.isZero()
        || initialBackoff.isNegative() || initialBackoff.compareTo(maxBackoff) > 0
        || maxBackoff.compareTo(Duration.ofSeconds(30)) > 0 || jitter.isNegative()
        || jitter.compareTo(maxBackoff) > 0) {
      throw new IllegalArgumentException("reconnect policy is outside supported bounds");
    }
  }

  public static ReconnectPolicy defaults() {
    return new ReconnectPolicy(5, Duration.ofMillis(25), Duration.ofSeconds(2),
        Duration.ofMillis(25), 0x684332000011L);
  }

  Duration delay(int attempt) {
    long multiplier = 1L << Math.min(20, Math.max(0, attempt - 1));
    long base = Math.min(maxBackoff.toNanos(), saturatedMultiply(initialBackoff.toNanos(), multiplier));
    if (jitter.isZero()) return Duration.ofNanos(base);
    long mixed = mix64(seed ^ attempt);
    long bound = jitter.toNanos();
    long offset = Long.remainderUnsigned(mixed, bound == Long.MAX_VALUE ? bound : bound + 1);
    return Duration.ofNanos(Math.min(maxBackoff.toNanos(), saturatedAdd(base, offset)));
  }

  private static long saturatedMultiply(long left, long right) {
    try { return Math.multiplyExact(left, right); }
    catch (ArithmeticException overflow) { return Long.MAX_VALUE; }
  }

  private static long saturatedAdd(long left, long right) {
    try { return Math.addExact(left, right); }
    catch (ArithmeticException overflow) { return Long.MAX_VALUE; }
  }

  private static long mix64(long value) {
    value = (value ^ (value >>> 30)) * 0xbf58476d1ce4e5b9L;
    value = (value ^ (value >>> 27)) * 0x94d049bb133111ebL;
    return value ^ (value >>> 31);
  }
}
