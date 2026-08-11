package io.hydracache.client.hc2;

/** Current single-key fenced-lock ownership metadata. */
public record LockOwnership(boolean locked, long fence) {
  public LockOwnership {
    if ((locked && fence <= 0) || (!locked && fence != 0)) {
      throw new IllegalArgumentException("fence must be positive exactly when locked");
    }
  }
}
