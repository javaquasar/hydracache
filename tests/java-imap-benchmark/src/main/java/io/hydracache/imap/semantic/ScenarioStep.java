package io.hydracache.imap.semantic;

import java.util.List;
import java.util.Objects;
import java.util.Optional;

/** One deterministic semantic operation. All collections are immutable and request ordered. */
public record ScenarioStep(
    String id,
    Operation operation,
    List<BytesValue> keys,
    List<ScenarioEntry> entries,
    Optional<BytesValue> expected,
    TtlDirective ttl,
    long advanceTicks) {
  public ScenarioStep {
    if (Objects.requireNonNull(id, "id").isBlank()) {
      throw new IllegalArgumentException("step id must not be blank");
    }
    Objects.requireNonNull(operation, "operation");
    keys = List.copyOf(keys);
    entries = List.copyOf(entries);
    expected = Objects.requireNonNull(expected, "expected");
    Objects.requireNonNull(ttl, "ttl");
    validateShape(operation, keys, entries, expected, advanceTicks);
  }

  public static ScenarioStep get(String id, BytesValue key) {
    return new ScenarioStep(id, Operation.GET, List.of(key), List.of(), Optional.empty(),
        TtlDirective.preserve(), 0);
  }

  public BytesValue key() {
    if (!keys.isEmpty()) return keys.get(0);
    if (!entries.isEmpty()) return entries.get(0).key();
    throw new IllegalStateException("step has no key");
  }

  public BytesValue value() {
    if (entries.isEmpty()) throw new IllegalStateException("step has no value");
    return entries.get(0).value();
  }

  private static void validateShape(
      Operation operation,
      List<BytesValue> keys,
      List<ScenarioEntry> entries,
      Optional<BytesValue> expected,
      long advanceTicks) {
    switch (operation) {
      case GET, GET_AND_REMOVE -> require(keys.size() == 1 && entries.isEmpty()
          && expected.isEmpty() && advanceTicks == 0, operation);
      case PUT, PUT_IF_ABSENT, GET_AND_PUT -> require(entries.size() == 1 && keys.isEmpty()
          && expected.isEmpty() && advanceTicks == 0, operation);
      case REPLACE -> require(entries.size() == 1 && keys.isEmpty()
          && expected.isPresent() && advanceTicks == 0, operation);
      case GET_ALL, REMOVE_ALL -> require(!keys.isEmpty() && entries.isEmpty()
          && expected.isEmpty() && advanceTicks == 0, operation);
      case PUT_ALL -> require(!entries.isEmpty() && keys.isEmpty()
          && expected.isEmpty() && advanceTicks == 0, operation);
      case ADVANCE -> require(keys.isEmpty() && entries.isEmpty()
          && expected.isEmpty() && advanceTicks > 0, operation);
    }
  }

  private static void require(boolean condition, Operation operation) {
    if (!condition) throw new IllegalArgumentException("invalid fields for " + operation);
  }
}
