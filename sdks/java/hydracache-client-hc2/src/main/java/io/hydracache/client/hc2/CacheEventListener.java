package io.hydracache.client.hc2;

/** Listener invoked on the configured callback executor. */
public interface CacheEventListener {
  void onEvent(CacheEvent event);
  void onGap(long subscriptionId, long afterWatermark);
}
