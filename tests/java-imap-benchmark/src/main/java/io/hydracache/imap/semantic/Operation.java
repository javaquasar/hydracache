package io.hydracache.imap.semantic;

/** Transport-neutral operations admitted by the provisional W11b semantic trace. */
public enum Operation {
  GET,
  PUT,
  PUT_IF_ABSENT,
  REPLACE,
  GET_AND_PUT,
  GET_AND_REMOVE,
  GET_ALL,
  PUT_ALL,
  REMOVE_ALL,
  ADVANCE
}
