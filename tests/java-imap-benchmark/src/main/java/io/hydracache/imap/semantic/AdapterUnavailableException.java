package io.hydracache.imap.semantic;

/** Product adapter invoked before its frozen compatibility boundary exists. */
public final class AdapterUnavailableException extends IllegalStateException {
  public AdapterUnavailableException(String message) {
    super(message);
  }
}
