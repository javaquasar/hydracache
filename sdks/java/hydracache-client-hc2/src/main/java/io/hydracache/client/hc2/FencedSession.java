package io.hydracache.client.hc2;

/** Server-issued session identity and monotonic fencing token. */
public interface FencedSession extends AutoCloseable {
  byte[] id();
  long fence();
  @Override void close();
}
