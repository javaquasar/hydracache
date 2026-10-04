package io.hydracache.imap.semantic;

/** Error taxonomy used by semantic admission before performance execution. */
public enum ErrorClass {
  NONE,
  SIZE,
  DEADLINE,
  AUTHORIZATION,
  UNSUPPORTED,
  PARTIAL,
  DUPLICATE_KEY,
  INTERNAL
}
