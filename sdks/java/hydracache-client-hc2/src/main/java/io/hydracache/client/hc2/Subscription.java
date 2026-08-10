package io.hydracache.client.hc2;

/** Active server subscription. Closing emits an HC/2 unsubscribe command exactly once. */
public interface Subscription extends AutoCloseable {
  long id();
  long initialWatermark();
  @Override void close();
}
