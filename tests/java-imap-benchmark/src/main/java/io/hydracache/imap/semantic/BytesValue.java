package io.hydracache.imap.semantic;

import java.util.Arrays;
import java.util.HexFormat;
import java.util.Objects;

/** Immutable byte value used at the common adapter boundary. */
public final class BytesValue implements Comparable<BytesValue> {
  private static final HexFormat HEX = HexFormat.of();
  private final byte[] value;

  private BytesValue(byte[] value) {
    this.value = value;
  }

  public static BytesValue copyOf(byte[] value) {
    return new BytesValue(Objects.requireNonNull(value, "value").clone());
  }

  public static BytesValue fromHex(String value) {
    Objects.requireNonNull(value, "value");
    if ((value.length() & 1) != 0) {
      throw new IllegalArgumentException("hex value must have even length");
    }
    try {
      return new BytesValue(HEX.parseHex(value));
    } catch (IllegalArgumentException error) {
      throw new IllegalArgumentException("invalid lowercase or uppercase hex value", error);
    }
  }

  public byte[] copy() {
    return value.clone();
  }

  public int size() {
    return value.length;
  }

  public String hex() {
    return HEX.formatHex(value);
  }

  @Override
  public int compareTo(BytesValue other) {
    return Arrays.compareUnsigned(value, other.value);
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof BytesValue that && Arrays.equals(value, that.value);
  }

  @Override
  public int hashCode() {
    return Arrays.hashCode(value);
  }

  @Override
  public String toString() {
    return hex();
  }
}
