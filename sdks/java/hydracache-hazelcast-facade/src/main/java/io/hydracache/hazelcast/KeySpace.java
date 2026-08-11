package io.hydracache.hazelcast;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

final class KeySpace {
  private static final byte[] MAGIC = new byte[] {'H', 'C', 2};
  private final byte[] prefix;

  KeySpace(byte kind, String name) {
    if (name == null || name.isBlank()) throw new IllegalArgumentException("name is required");
    byte[] encoded = name.getBytes(StandardCharsets.UTF_8);
    if (encoded.length > 1024) throw new IllegalArgumentException("name exceeds 1024 UTF-8 bytes");
    prefix = ByteBuffer.allocate(MAGIC.length + 1 + 2 + encoded.length)
        .put(MAGIC).put(kind).putShort((short) encoded.length).put(encoded).array();
  }

  byte[] prefix() { return prefix.clone(); }

  byte[] key(byte[] encodedKey) {
    if (encodedKey == null || encodedKey.length == 0) {
      throw new IllegalArgumentException("encoded key must not be empty");
    }
    if ((long) prefix.length + encodedKey.length > 1024 * 1024) {
      throw new IllegalArgumentException("namespaced key exceeds HC/2 limit");
    }
    byte[] result = Arrays.copyOf(prefix, prefix.length + encodedKey.length);
    System.arraycopy(encodedKey, 0, result, prefix.length, encodedKey.length);
    return result;
  }

  byte[] decode(byte[] physicalKey) {
    if (physicalKey.length <= prefix.length
        || !Arrays.equals(prefix, Arrays.copyOf(physicalKey, prefix.length))) {
      throw new HydraCacheFacadeException("listener returned a key outside its map namespace");
    }
    return Arrays.copyOfRange(physicalKey, prefix.length, physicalKey.length);
  }
}
