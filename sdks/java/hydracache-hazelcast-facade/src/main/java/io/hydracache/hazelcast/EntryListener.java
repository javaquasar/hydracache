package io.hydracache.hazelcast;

/** Listener for best-effort cache repair signals. */
public interface EntryListener<K, V> {
  void onEvent(EntryEvent<K, V> event);
  void onGap(long afterWatermark);
}
