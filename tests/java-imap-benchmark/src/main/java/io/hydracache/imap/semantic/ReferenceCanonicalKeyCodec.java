package io.hydracache.imap.semantic;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Objects;

/** Provisional reference framing only; this is not a production wire or partition identity. */
public final class ReferenceCanonicalKeyCodec {
  private static final byte[] DOMAIN = new byte[] {'H', 'C', 'R', 'K', '0', '7', '5', 0};

  public record Bounds(int maxTenantBytes, int maxNamespaceBytes, int maxKeyBytes,
                       int maxFrameBytes) {
    public Bounds {
      if (maxTenantBytes <= 0 || maxNamespaceBytes <= 0 || maxKeyBytes <= 0
          || maxFrameBytes <= 0) {
        throw new IllegalArgumentException("reference key bounds must be positive");
      }
    }

    public static Bounds defaults() {
      return new Bounds(256, 256, 1024 * 1024, 1024 * 1024 + 1024);
    }
  }

  public record Key(String tenant, String namespace, long generation, byte[] key) {
    public Key {
      Objects.requireNonNull(tenant, "tenant");
      Objects.requireNonNull(namespace, "namespace");
      key = Objects.requireNonNull(key, "key").clone();
      if (tenant.isEmpty() || namespace.isEmpty() || generation <= 0) {
        throw new IllegalArgumentException("tenant, namespace, and generation are required");
      }
    }

    @Override public byte[] key() { return key.clone(); }

    @Override
    public boolean equals(Object other) {
      return other instanceof Key that && tenant.equals(that.tenant)
          && namespace.equals(that.namespace) && generation == that.generation
          && Arrays.equals(key, that.key);
    }

    @Override public int hashCode() {
      return Objects.hash(tenant, namespace, generation, Arrays.hashCode(key));
    }
  }

  private ReferenceCanonicalKeyCodec() {}

  public static byte[] encode(Key key, Bounds bounds) {
    Objects.requireNonNull(key, "key");
    Objects.requireNonNull(bounds, "bounds");
    byte[] tenant = key.tenant().getBytes(StandardCharsets.UTF_8);
    byte[] namespace = key.namespace().getBytes(StandardCharsets.UTF_8);
    requireLength("tenant", tenant.length, bounds.maxTenantBytes());
    requireLength("namespace", namespace.length, bounds.maxNamespaceBytes());
    requireLength("key", key.key().length, bounds.maxKeyBytes());
    long size = (long) DOMAIN.length + 4 + tenant.length + 4 + namespace.length + 8 + 4
        + key.key().length;
    requireLength("frame", size, bounds.maxFrameBytes());
    ByteBuffer frame = ByteBuffer.allocate(Math.toIntExact(size)).order(ByteOrder.BIG_ENDIAN);
    frame.put(DOMAIN);
    put(frame, tenant);
    put(frame, namespace);
    frame.putLong(key.generation());
    put(frame, key.key());
    return frame.array();
  }

  public static Key decode(byte[] encoded, Bounds bounds) {
    Objects.requireNonNull(encoded, "encoded");
    Objects.requireNonNull(bounds, "bounds");
    requireLength("frame", encoded.length, bounds.maxFrameBytes());
    ByteBuffer frame = ByteBuffer.wrap(encoded).order(ByteOrder.BIG_ENDIAN);
    if (frame.remaining() < DOMAIN.length) throw malformed();
    byte[] domain = new byte[DOMAIN.length];
    frame.get(domain);
    if (!Arrays.equals(domain, DOMAIN)) throw malformed();
    String tenant = decodeUtf8(take(frame, "tenant", bounds.maxTenantBytes()));
    String namespace = decodeUtf8(take(frame, "namespace", bounds.maxNamespaceBytes()));
    if (frame.remaining() < Long.BYTES) throw malformed();
    long generation = frame.getLong();
    byte[] key = take(frame, "key", bounds.maxKeyBytes());
    if (frame.hasRemaining()) throw malformed();
    Key decoded = new Key(tenant, namespace, generation, key);
    if (!Arrays.equals(encoded, encode(decoded, bounds))) throw malformed();
    return decoded;
  }

  private static void put(ByteBuffer target, byte[] value) {
    target.putInt(value.length);
    target.put(value);
  }

  private static byte[] take(ByteBuffer source, String field, int limit) {
    if (source.remaining() < Integer.BYTES) throw malformed();
    long length = Integer.toUnsignedLong(source.getInt());
    requireLength(field, length, limit);
    if (length > source.remaining()) throw malformed();
    byte[] value = new byte[(int) length];
    source.get(value);
    return value;
  }

  private static String decodeUtf8(byte[] value) {
    try {
      return StandardCharsets.UTF_8.newDecoder()
          .onMalformedInput(CodingErrorAction.REPORT)
          .onUnmappableCharacter(CodingErrorAction.REPORT)
          .decode(ByteBuffer.wrap(value)).toString();
    } catch (CharacterCodingException error) {
      throw malformed();
    }
  }

  private static void requireLength(String field, long actual, int limit) {
    if (actual > limit) {
      throw new IllegalArgumentException(field + " exceeds limit=" + limit + ", actual=" + actual);
    }
  }

  private static IllegalArgumentException malformed() {
    return new IllegalArgumentException("malformed reference canonical key");
  }
}
