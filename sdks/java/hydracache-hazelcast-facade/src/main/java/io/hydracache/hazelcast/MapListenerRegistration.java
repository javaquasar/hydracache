package io.hydracache.hazelcast;

/** Active map-listener registration. */
@FunctionalInterface
public interface MapListenerRegistration extends AutoCloseable {
  @Override void close();
}
