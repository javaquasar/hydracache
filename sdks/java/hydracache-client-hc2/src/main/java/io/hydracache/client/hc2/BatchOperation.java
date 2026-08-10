package io.hydracache.client.hc2;

import java.time.Duration;
import java.util.Objects;

/** Typed operation accepted by {@link HydraCacheClient#batch}. */
public sealed interface BatchOperation {
  int MAX_KEY_BYTES = 1024 * 1024;
  int MAX_VALUE_BYTES = 16 * 1024 * 1024;
  Duration MAX_TTL = Duration.ofDays(3650);

  byte[] key();

  record Get(byte[] key) implements BatchOperation {
    public Get { key = copyBytes(key, "key", 1, MAX_KEY_BYTES); }
    @Override public byte[] key() { return key.clone(); }
  }

  record Put(byte[] key, byte[] value, Duration ttl) implements BatchOperation {
    public Put {
      key = copyBytes(key, "key", 1, MAX_KEY_BYTES);
      value = copyBytes(value, "value", 0, MAX_VALUE_BYTES);
      ttl = validTtl(ttl);
    }
    @Override public byte[] key() { return key.clone(); }
    @Override public byte[] value() { return value.clone(); }
  }

  record Delete(byte[] key) implements BatchOperation {
    public Delete { key = copyBytes(key, "key", 1, MAX_KEY_BYTES); }
    @Override public byte[] key() { return key.clone(); }
  }

  record CompareAndSet(byte[] key, byte[] expected, byte[] replacement, Duration ttl)
      implements BatchOperation {
    public CompareAndSet {
      key = copyBytes(key, "key", 1, MAX_KEY_BYTES);
      expected = copyBytes(expected, "expected", 0, MAX_VALUE_BYTES);
      replacement = copyBytes(replacement, "replacement", 0, MAX_VALUE_BYTES);
      ttl = validTtl(ttl);
    }
    @Override public byte[] key() { return key.clone(); }
    @Override public byte[] expected() { return expected.clone(); }
    @Override public byte[] replacement() { return replacement.clone(); }
  }

  private static byte[] copyBytes(byte[] value, String name, int minimum, int maximum) {
    Objects.requireNonNull(value, name);
    if (value.length < minimum || value.length > maximum) {
      throw new IllegalArgumentException(
          name + " length must be in [" + minimum + ", " + maximum + "]");
    }
    return value.clone();
  }

  private static Duration validTtl(Duration ttl) {
    if (ttl != null && (ttl.isNegative() || ttl.compareTo(MAX_TTL) > 0)) {
      throw new IllegalArgumentException("TTL must be nonnegative and at most ten years");
    }
    return ttl;
  }
}
