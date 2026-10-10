package io.hydracache.imap.semantic;

import java.nio.ByteBuffer;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import java.util.Optional;
import java.util.function.Predicate;

/** Reproducible bounded scenario generation and deterministic deletion shrinking. */
public final class SeededSemanticScenario {
  private SeededSemanticScenario() {}

  public static ScenarioManifest generate(long seed, int stepCount) {
    if (stepCount <= 0 || stepCount > 10_000) {
      throw new IllegalArgumentException("stepCount must be in 1..10000");
    }
    long state = seed;
    var steps = new ArrayList<ScenarioStep>(stepCount);
    for (int index = 0; index < stepCount; index++) {
      state = next(state);
      BytesValue key = byteValue((int) (state & 3));
      BytesValue value = byteValue((int) ((state >>> 8) & 7) + 16);
      BytesValue expected = byteValue((int) ((state >>> 16) & 7) + 16);
      String id = "seed-" + index;
      steps.add(switch (index % 9) {
        case 0 -> write(id, Operation.PUT, key, value, Optional.empty(),
            TtlDirective.expireAfter(2));
        case 1 -> write(id, Operation.PUT_IF_ABSENT, key, value, Optional.empty(),
            TtlDirective.eternal());
        case 2 -> write(id, Operation.REPLACE, key, value, Optional.of(expected),
            TtlDirective.preserve());
        case 3 -> new ScenarioStep(id, Operation.REMOVE_IF_VALUE, List.of(key), List.of(),
            Optional.of(expected), TtlDirective.preserve(), 0);
        case 4 -> write(id, Operation.GET_AND_PUT, key, value, Optional.empty(),
            TtlDirective.preserve());
        case 5 -> new ScenarioStep(id, Operation.SET_TTL, List.of(key), List.of(), Optional.empty(),
            TtlDirective.expireAfter(1), 0);
        case 6 -> new ScenarioStep(id, Operation.ADVANCE, List.of(), List.of(), Optional.empty(),
            TtlDirective.preserve(), 1);
        case 7 -> new ScenarioStep(id, Operation.GET_ALL,
            List.of(key, byteValue(((int) (state >>> 24) & 3) + 4)), List.of(), Optional.empty(),
            TtlDirective.preserve(), 0);
        default -> new ScenarioStep(id, Operation.LISTENER_GAP, List.of(key), List.of(),
            Optional.empty(), TtlDirective.preserve(), 0);
      });
    }
    return new ScenarioManifest(seed, steps);
  }

  public static List<ScenarioStep> shrink(
      List<ScenarioStep> original, Predicate<List<ScenarioStep>> stillFails) {
    var minimized = new ArrayList<>(List.copyOf(original));
    int index = 0;
    while (index < minimized.size()) {
      var candidate = new ArrayList<>(minimized);
      candidate.remove(index);
      if (stillFails.test(List.copyOf(candidate))) {
        minimized = candidate;
      } else {
        index++;
      }
    }
    return List.copyOf(minimized);
  }

  public static String fingerprint(ScenarioManifest scenario) {
    MessageDigest digest;
    try {
      digest = MessageDigest.getInstance("SHA-256");
    } catch (NoSuchAlgorithmException error) {
      throw new IllegalStateException("SHA-256 unavailable", error);
    }
    digest.update(ByteBuffer.allocate(Long.BYTES).putLong(scenario.seed()).array());
    for (ScenarioStep step : scenario.steps()) {
      digest.update(step.id().getBytes(java.nio.charset.StandardCharsets.UTF_8));
      digest.update((byte) step.operation().ordinal());
      for (BytesValue key : step.keys()) digest.update(key.copy());
      for (ScenarioEntry entry : step.entries()) {
        digest.update(entry.key().copy());
        digest.update(entry.value().copy());
      }
      step.expected().ifPresent(value -> digest.update(value.copy()));
      digest.update(ByteBuffer.allocate(Long.BYTES).putLong(step.advanceTicks()).array());
    }
    return HexFormat.of().formatHex(digest.digest());
  }

  private static ScenarioStep write(
      String id,
      Operation operation,
      BytesValue key,
      BytesValue value,
      Optional<BytesValue> expected,
      TtlDirective ttl) {
    return new ScenarioStep(id, operation, List.of(), List.of(new ScenarioEntry(key, value)),
        expected, ttl, 0);
  }

  private static BytesValue byteValue(int value) {
    return BytesValue.copyOf(new byte[] {(byte) value});
  }

  private static long next(long state) {
    long value = state == 0 ? 0x9e3779b97f4a7c15L : state;
    value ^= value << 13;
    value ^= value >>> 7;
    value ^= value << 17;
    return value;
  }
}
