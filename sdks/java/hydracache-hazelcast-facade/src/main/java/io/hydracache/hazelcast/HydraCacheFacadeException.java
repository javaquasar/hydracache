package io.hydracache.hazelcast;

/** Local facade error that preserves the HC/2 cause when one exists. */
public final class HydraCacheFacadeException extends RuntimeException {
  public HydraCacheFacadeException(String message) { super(message); }
  public HydraCacheFacadeException(String message, Throwable cause) { super(message, cause); }
}
