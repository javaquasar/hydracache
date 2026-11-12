package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.*;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

final class SemanticScenarioFingerprintTest {
  private static BytesValue b(String hex) { return BytesValue.fromHex(hex); }
  private static ScenarioManifest put(String key, String value, TtlDirective ttl) {
    return new ScenarioManifest(117, List.of(new ScenarioStep("same", Operation.PUT, List.of(),
        List.of(new ScenarioEntry(b(key), b(value))), Optional.empty(), ttl, 0)));
  }

  @Test void canonicalVersionSeparatesFieldBoundariesTtlPresenceAndStableOperationNames() {
    var a = put("61", "6263", TtlDirective.eternal());
    var b = put("6162", "63", TtlDirective.eternal());
    assertEquals(SeededSemanticScenario.legacyFingerprintV1(a), SeededSemanticScenario.legacyFingerprintV1(b));
    assertNotEquals(SeededSemanticScenario.fingerprint(a), SeededSemanticScenario.fingerprint(b));
    for (var ttl : List.of(TtlDirective.preserve(), TtlDirective.expireAfter(1), TtlDirective.expireAfter(2))) {
      assertNotEquals(SeededSemanticScenario.fingerprint(a), SeededSemanticScenario.fingerprint(put("61", "6263", ttl)));
    }
    var emptyExpected = new ScenarioManifest(117, List.of(new ScenarioStep("same", Operation.REMOVE_IF_VALUE,
        List.of(b("61")), List.of(), Optional.of(b("")), TtlDirective.preserve(), 0)));
    var valueExpected = new ScenarioManifest(117, List.of(new ScenarioStep("same", Operation.REMOVE_IF_VALUE,
        List.of(b("61")), List.of(), Optional.of(b("00")), TtlDirective.preserve(), 0)));
    assertNotEquals(SeededSemanticScenario.fingerprint(emptyExpected), SeededSemanticScenario.fingerprint(valueExpected));
    assertTrue(SeededSemanticScenario.fingerprint(a).matches("v2:[0-9a-f]{64}"));
    assertEquals("put", SemanticScenarioFingerprint.operationName(Operation.PUT));
    assertEquals("remove_if_value", SemanticScenarioFingerprint.operationName(Operation.REMOVE_IF_VALUE));
    assertTrue(SemanticScenarioFingerprint.verify(a, SeededSemanticScenario.fingerprint(a)));
    assertFalse(SemanticScenarioFingerprint.verify(a, SeededSemanticScenario.fingerprint(b)));
  }

  @Test void legacyMigrationIsExplicitAndFutureOrMalformedIdentitiesFailClosed() {
    var scenario = SeededSemanticScenario.generate(117, 90);
    String old = "08cd0f692503e5105b115e71a62f165ebdc3ebb3aaf1376d06f0a5ca8e956bfe";
    assertEquals(old, SeededSemanticScenario.legacyFingerprintV1(scenario));
    assertEquals("v2:6b84b03698a794d6ce556af4a25f511b232cfae72266ef50f6d484f564b36b12", SeededSemanticScenario.fingerprint(scenario));
    assertEquals("v2:92c54dd48339f566e25c789078d6de11b3f22e9db1627844ab5ae1bf975d571f", SeededSemanticScenario.fingerprint(SeededSemanticScenario.generate(0, 1)));
    assertEquals("v2:3d81c0420a91f8695c6fd5f27469c93754e41889d30b4a6d8ff532a6433b6b3c", SeededSemanticScenario.fingerprint(SeededSemanticScenario.generate(-1, 18)));
    assertEquals(SeededSemanticScenario.fingerprint(scenario), SemanticScenarioFingerprint.migrateLegacy(scenario, old));
    assertThrows(IllegalArgumentException.class, () -> SemanticScenarioFingerprint.migrateLegacy(scenario, "0".repeat(64)));
    for (String invalid : List.of(old, "v3:" + old, "v2:short", "v2:" + old.toUpperCase())) {
      assertThrows(IllegalArgumentException.class, () -> SemanticScenarioFingerprint.verify(scenario, invalid));
    }
    assertThrows(IllegalArgumentException.class, () -> SemanticScenarioFingerprint.migrateLegacy(scenario, "v1:" + old));
    assertThrows(IllegalStateException.class, () -> SemanticScenarioFingerprint.digest(scenario, "missing-digest"));
  }

  @Test void canonicalPreflightBoundsDirectTypedInputsBeforeDigestAllocation() {
    var giantId = new ScenarioManifest(1, List.of(ScenarioStep.get("x".repeat(257), b(""))));
    assertThrows(IllegalArgumentException.class, () -> SeededSemanticScenario.fingerprint(giantId));
    for (String invalidId : List.of("é".repeat(129), "x\uD800")) assertThrows(IllegalArgumentException.class,
        () -> SeededSemanticScenario.fingerprint(new ScenarioManifest(1, List.of(ScenarioStep.get(invalidId, b(""))))));
    assertNotEquals(SeededSemanticScenario.fingerprint(new ScenarioManifest(1, List.of(ScenarioStep.get("ключ😀", b(""))))),
        SeededSemanticScenario.fingerprint(new ScenarioManifest(1, List.of(ScenarioStep.get("key", b(""))))));
    var giantValue = new ScenarioManifest(1, List.of(new ScenarioStep("giant", Operation.PUT, List.of(),
        List.of(new ScenarioEntry(b(""), BytesValue.copyOf(new byte[1024 * 1024 + 1]))), Optional.empty(), TtlDirective.eternal(), 0)));
    assertThrows(IllegalArgumentException.class, () -> SeededSemanticScenario.fingerprint(giantValue));
    var steps = java.util.stream.IntStream.range(0, 10_001).mapToObj(i -> ScenarioStep.get("s" + i, b(""))).toList();
    assertThrows(IllegalArgumentException.class, () -> SeededSemanticScenario.fingerprint(new ScenarioManifest(1, steps)));
    var keys = java.util.Collections.nCopies(257, b(""));
    assertThrows(IllegalArgumentException.class, () -> SeededSemanticScenario.fingerprint(new ScenarioManifest(1,
        List.of(new ScenarioStep("many", Operation.GET_ALL, keys, List.of(), Optional.empty(), TtlDirective.preserve(), 0)))));
    assertThrows(IllegalArgumentException.class, () -> SeededSemanticScenario.fingerprint(new ScenarioManifest(1,
        List.of(new ScenarioStep("many", Operation.PUT_ALL, List.of(), java.util.Collections.nCopies(257, new ScenarioEntry(b(""), b(""))), Optional.empty(), TtlDirective.eternal(), 0)))));
    var bytes = BytesValue.copyOf(new byte[1024 * 1024]);
    var many = java.util.stream.IntStream.range(0, 17).mapToObj(i -> new ScenarioStep("payload" + i,
        Operation.PUT, List.of(), List.of(new ScenarioEntry(b(""), bytes)), Optional.empty(), TtlDirective.eternal(), 0)).toList();
    assertThrows(IllegalArgumentException.class, () -> SeededSemanticScenario.fingerprint(new ScenarioManifest(1, many)));
  }

  @Test void shrinkCountsInitialReplayAndStopsBeforeReplayOrWorkBudgetsAreExceeded() {
    var original = SeededSemanticScenario.generate(7, 18).steps();
    var calls = new AtomicInteger();
    var budget = new SeededSemanticScenario.ShrinkBudget(1, 100);
    var stopped = SeededSemanticScenario.shrink(original, budget, candidate -> { calls.incrementAndGet(); return true; });
    assertEquals(1, calls.get()); assertEquals(1, stopped.replays()); assertEquals(18, stopped.work());
    assertEquals(original, stopped.steps()); assertTrue(stopped.budgetExhausted());
    var workStopped = SeededSemanticScenario.shrink(original, new SeededSemanticScenario.ShrinkBudget(64, 18), candidate -> true);
    assertEquals(1, workStopped.replays()); assertTrue(workStopped.budgetExhausted());
    var result = SeededSemanticScenario.shrink(original, new SeededSemanticScenario.ShrinkBudget(64, 1024),
        candidate -> candidate.stream().anyMatch(step -> step.operation() == Operation.SET_TTL));
    assertEquals(1, result.steps().size()); assertFalse(result.budgetExhausted());
    assertThrows(UnsupportedOperationException.class, () -> result.steps().clear());
    var empty = SeededSemanticScenario.shrink(List.of(), new SeededSemanticScenario.ShrinkBudget(1, 1), candidate -> true);
    assertTrue(empty.steps().isEmpty()); assertFalse(empty.budgetExhausted());
    assertThrows(IllegalArgumentException.class, () -> SeededSemanticScenario.shrink(original, budget, candidate -> false));
    assertThrows(IllegalArgumentException.class, () -> SeededSemanticScenario.shrink(original, new SeededSemanticScenario.ShrinkBudget(2, 1), candidate -> true));
    for (int invalid : List.of(0, 1025)) assertThrows(IllegalArgumentException.class, () -> new SeededSemanticScenario.ShrinkBudget(invalid, 1));
    for (long invalid : List.of(0L, 10_000_001L)) assertThrows(IllegalArgumentException.class, () -> new SeededSemanticScenario.ShrinkBudget(1, invalid));
  }
}
