package io.hydracache.client.hc2;

/** Result of a mutation or compare-and-set operation. */
public record MutationResult(boolean applied) {}
