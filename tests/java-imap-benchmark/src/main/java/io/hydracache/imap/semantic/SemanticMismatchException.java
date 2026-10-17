package io.hydracache.imap.semantic;

/** Raised when a semantic-red cell attempts to proceed. */
public final class SemanticMismatchException extends IllegalStateException {
  public SemanticMismatchException(String message) { super(message); }
}
