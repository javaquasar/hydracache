package io.hydracache.hazelcast;

import java.nio.charset.StandardCharsets;

/** Explicit application codec. Java serialization is intentionally unsupported. */
public interface HydraCodec<T> {
  byte[] encode(T value);
  T decode(byte[] value);

  static HydraCodec<String> utf8() {
    return new HydraCodec<>() {
      @Override public byte[] encode(String value) {
        if (value == null) throw new NullPointerException("value");
        return value.getBytes(StandardCharsets.UTF_8);
      }
      @Override public String decode(byte[] value) {
        if (value == null) throw new NullPointerException("value");
        return new String(value, StandardCharsets.UTF_8);
      }
    };
  }

  static HydraCodec<byte[]> bytes() {
    return new HydraCodec<>() {
      @Override public byte[] encode(byte[] value) { return value.clone(); }
      @Override public byte[] decode(byte[] value) { return value.clone(); }
    };
  }
}
