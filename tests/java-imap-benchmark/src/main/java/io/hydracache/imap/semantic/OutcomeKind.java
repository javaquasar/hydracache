package io.hydracache.imap.semantic;

/** Stable semantic outcome classes, independent of either product transport. */
public enum OutcomeKind {
  PRESENT,
  ABSENT,
  INSERTED,
  REPLACED,
  REMOVED,
  MISMATCH,
  BULK,
  ADVANCED,
  ERROR
}
