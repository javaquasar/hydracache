package io.hydracache.hazelcast;

import io.hydracache.client.hc2.BatchItemOutcome;
import io.hydracache.client.hc2.BatchOperation;
import io.hydracache.client.hc2.CacheEvent;
import io.hydracache.client.hc2.CacheEventListener;
import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.RequestOptions;
import io.hydracache.client.hc2.Subscription;
import io.hydracache.client.hc2.SubscriptionGap;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
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

  /** Read keys in input order, including duplicate keys and partial owner failures. */
  public List<MapBatchItem<K, V>> getAll(List<K> keys, RequestOptions options) {
    List<K> ordered = List.copyOf(keys);
    List<BatchOperation> operations = ordered.stream()
        .<BatchOperation>map(key -> new BatchOperation.Get(physicalKey(key))).toList();
    return executeBatch(ordered, operations, options);
  }

  /** Store persistent values, preserving the input map's iteration order. */
  public List<MapBatchItem<K, V>> putAll(Map<K, V> entries, RequestOptions options) {
    var keys = new ArrayList<K>(entries.size());
    var operations = new ArrayList<BatchOperation>(entries.size());
    entries.forEach((key, value) -> {
      keys.add(key);
      operations.add(new BatchOperation.Put(physicalKey(key), valueCodec.encode(value), Duration.ZERO));
    });
    return executeBatch(keys, operations, options);
  }

  /** Remove keys in input order and preserve every outcome. */
  public List<MapBatchItem<K, V>> removeAll(List<K> keys, RequestOptions options) {
    List<K> ordered = List.copyOf(keys);
    List<BatchOperation> operations = ordered.stream()
        .<BatchOperation>map(key -> new BatchOperation.Delete(physicalKey(key))).toList();
    return executeBatch(ordered, operations, options);
  }

  private List<MapBatchItem<K, V>> executeBatch(
      List<K> keys, List<BatchOperation> operations, RequestOptions options) {
    if (operations.isEmpty()) return List.of();
    List<BatchItemOutcome> outcomes = await(client.batchOutcomes(
        operations, options == null ? options() : options));
    if (outcomes.size() != keys.size()) {
      throw new HydraCacheFacadeException("HC/2 batch outcome count did not match the input");
    }
    var result = new ArrayList<MapBatchItem<K, V>>(keys.size());
    for (int index = 0; index < outcomes.size(); index++) {
      BatchItemOutcome outcome = outcomes.get(index);
      Optional<V> value = outcome.result().flatMap(item -> item.value())
          .map(item -> valueCodec.decode(item.value()));
      result.add(new MapBatchItem<>(keys.get(index), value, outcome));
    }
    return List.copyOf(result);
  }

  public MapListenerRegistration addEntryListener(EntryListener<K, V> listener) {
    return addEntryListener(0, listener);
  }

  public MapListenerRegistration addEntryListener(long resumeWatermark, EntryListener<K, V> listener) {
    Objects.requireNonNull(listener, "listener");
    Subscription subscription = await(client.subscribe(
        keySpace.prefix(), resumeWatermark, true, new CacheEventListener() {
          @Override public void onEvent(CacheEvent event) {
            K key = keyCodec.decode(keySpace.decode(event.key()));
            Optional<V> value = event.removed()
                ? Optional.empty() : Optional.of(valueCodec.decode(event.value()));
            listener.onEvent(new EntryEvent<>(key, value, event.removed(), event.watermark()));
          }

          @Override public void onGap(long subscriptionId, long afterWatermark) {
            listener.onGap(afterWatermark);
          }

          @Override public void onGap(long subscriptionId, long afterWatermark, SubscriptionGap gap) {
            listener.onGap(afterWatermark, gap);
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
