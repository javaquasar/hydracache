package io.hydracache.imap.semantic;

/** Fail-closed scenario parsing error. */
public final class ManifestException extends Exception {
  public ManifestException(String message) {
    super(message);
  }

  public ManifestException(String message, Throwable cause) {
    super(message, cause);
  }
}
