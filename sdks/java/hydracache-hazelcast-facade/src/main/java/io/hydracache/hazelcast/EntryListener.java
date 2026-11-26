package io.hydracache.hazelcast;

import io.hydracache.client.hc2.SubscriptionGap;

/** Listener for best-effort cache repair signals. */
public interface EntryListener<K, V> {
  void onEvent(EntryEvent<K, V> event);
  void onGap(long afterWatermark);

  default void onGap(long afterWatermark, SubscriptionGap gap) {
    onGap(afterWatermark);
  }
}
