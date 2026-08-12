package io.hydracache.hazelcast;

import io.hydracache.client.hc2.CacheEvent;
import io.hydracache.client.hc2.CacheEventListener;
import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.RequestOptions;
import io.hydracache.client.hc2.Subscription;
import java.time.Duration;
import java.util.Objects;
import java.util.Optional;
import java.util.concurrent.CompletionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

/** Narrow source-level IMap analogue backed by HC/2 bytes and explicit codecs. */
public final class HydraMap<K, V> {
  private final HydraCacheClient client;
  private final KeySpace keySpace;
  private final HydraCodec<K> keyCodec;
  private final HydraCodec<V> valueCodec;
  private final Duration requestTimeout;

  HydraMap(
      HydraCacheClient client,
      String name,
      HydraCodec<K> keyCodec,
      HydraCodec<V> valueCodec,
      Duration requestTimeout) {
    this.client = Objects.requireNonNull(client, "client");
    this.keySpace = new KeySpace((byte) 1, name);
    this.keyCodec = Objects.requireNonNull(keyCodec, "keyCodec");
    this.valueCodec = Objects.requireNonNull(valueCodec, "valueCodec");
    this.requestTimeout = HydraCacheInstance.requirePositive(requestTimeout, "requestTimeout");
  }

  public Optional<V> get(K key) {
    return await(client.get(physicalKey(key), options()))
        .map(value -> valueCodec.decode(value.value()));
  }

  public void put(K key, V value) { put(key, value, Duration.ZERO); }

  public void put(K key, V value, Duration ttl) {
    Objects.requireNonNull(ttl, "ttl");
    if (ttl.isNegative()) throw new IllegalArgumentException("ttl must be nonnegative");
    var result = await(client.put(
        physicalKey(key), valueCodec.encode(value), ttl, options()));
    if (!result.applied()) throw new HydraCacheFacadeException("put was not applied");
  }

  public boolean replace(K key, V expected, V replacement) {
    return replace(key, expected, replacement, Duration.ZERO);
  }

  public boolean replace(K key, V expected, V replacement, Duration ttl) {
    Objects.requireNonNull(ttl, "ttl");
    if (ttl.isNegative()) throw new IllegalArgumentException("ttl must be nonnegative");
    return await(client.compareAndSet(
        physicalKey(key), valueCodec.encode(expected), valueCodec.encode(replacement),
        ttl, options())).applied();
  }

  public boolean remove(K key, V expected) {
    return await(client.removeIfValue(
        physicalKey(key), valueCodec.encode(expected), options())).applied();
  }

  public MapListenerRegistration addEntryListener(EntryListener<K, V> listener) {
    Objects.requireNonNull(listener, "listener");
    Subscription subscription = await(client.subscribe(
        keySpace.prefix(), 0, new CacheEventListener() {
          @Override public void onEvent(CacheEvent event) {
            K key = keyCodec.decode(keySpace.decode(event.key()));
            Optional<V> value = event.removed()
                ? Optional.empty() : Optional.of(valueCodec.decode(event.value()));
            listener.onEvent(new EntryEvent<>(key, value, event.removed(), event.watermark()));
          }

          @Override public void onGap(long subscriptionId, long afterWatermark) {
            listener.onGap(afterWatermark);
          }
        }));
    return subscription::close;
  }

  public HydraFencedLock<K> getLock(K key) {
    return new HydraFencedLock<>(client, physicalKey(key), requestTimeout);
  }

  public void executeOnKey(K key) {
    throw HydraCacheInstance.unsupported("IMap.executeOnKey");
  }

  private byte[] physicalKey(K key) { return keySpace.key(keyCodec.encode(key)); }
  private RequestOptions options() { return RequestOptions.withTimeout(requestTimeout); }

  private <T> T await(java.util.concurrent.CompletableFuture<T> future) {
    try {
      return future.get(requestTimeout.toMillis(), TimeUnit.MILLISECONDS);
    } catch (TimeoutException error) {
      future.cancel(true);
      throw new HydraCacheFacadeException("HC/2 operation exceeded the facade deadline", error);
    } catch (InterruptedException error) {
      Thread.currentThread().interrupt();
      throw new HydraCacheFacadeException("HC/2 operation was interrupted", error);
    } catch (java.util.concurrent.ExecutionException error) {
      Throwable cause = error.getCause();
      if (cause instanceof RuntimeException runtime) throw runtime;
      throw new CompletionException(cause);
    }
  }
}
