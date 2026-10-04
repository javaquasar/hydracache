package io.hydracache.imap.semantic;

/** Transport-neutral operations admitted by the provisional W11b semantic trace. */
public enum Operation {
  GET,
  CONTAINS_KEY,
  PUT,
  PUT_IF_ABSENT,
  REPLACE,
  REPLACE_IF_PRESENT,
  GET_AND_PUT,
  GET_AND_REMOVE,
  REMOVE_IF_VALUE,
  GET_ALL,
  PUT_ALL,
  REMOVE_ALL,
  SET_TTL,
  REMAINING_TTL,
  LISTENER_GAP,
  ADVANCE
}
