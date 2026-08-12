package io.hydracache.hazelcast;

import io.hydracache.client.hc2.HydraCacheClient;
import java.time.Duration;
import java.util.Objects;

/** Entry point for narrow Hazelcast-shaped map and fenced-lock migration APIs. */
public final class HydraCacheInstance implements AutoCloseable {
  private final HydraCacheClient client;
  private final Duration requestTimeout;
  private final boolean closeClient;

  public HydraCacheInstance(HydraCacheClient client, Duration requestTimeout) {
    this(client, requestTimeout, false);
  }

  public HydraCacheInstance(
      HydraCacheClient client, Duration requestTimeout, boolean closeClient) {
    this.client = Objects.requireNonNull(client, "client");
    this.requestTimeout = requirePositive(requestTimeout, "requestTimeout");
    this.closeClient = closeClient;
  }

  public <K, V> HydraMap<K, V> getMap(
      String name, HydraCodec<K> keyCodec, HydraCodec<V> valueCodec) {
    return new HydraMap<>(client, name, keyCodec, valueCodec, requestTimeout);
  }

  public <K> HydraFencedLock<K> getFencedLock(
      String name, K key, HydraCodec<K> keyCodec) {
    return new HydraFencedLock<>(
        client, new KeySpace((byte) 2, name).key(keyCodec.encode(key)), requestTimeout);
  }

  public static UnsupportedOperationException unsupported(String api) {
    return new UnsupportedOperationException(
        api + " is outside the HydraCache Hazelcast migration subset; "
            + "see META-INF/hydracache/hazelcast-capabilities.properties");
  }

  @Override public void close() {
    if (closeClient) client.close();
  }

  static Duration requirePositive(Duration value, String name) {
    Objects.requireNonNull(value, name);
    if (value.isZero() || value.isNegative()) {
      throw new IllegalArgumentException(name + " must be positive");
    }
    return value;
  }
}
