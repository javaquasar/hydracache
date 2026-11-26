package io.hydracache.hazelcast;

import io.hydracache.client.hc2.BatchItemOutcome;
import java.util.Optional;

/** A map key, its decoded read value when found, and the underlying HC/2 outcome. */
public record MapBatchItem<K, V>(K key, Optional<V> value, BatchItemOutcome outcome) {}
