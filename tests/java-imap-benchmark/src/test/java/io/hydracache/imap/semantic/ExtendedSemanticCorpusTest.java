package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedSemanticScenario.*;
import static org.junit.jupiter.api.Assertions.*;
import java.util.List;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

final class ExtendedSemanticCorpusTest {
  @Test void seededClosedCorpusRoundtripsAllActionsAndIndependentModelExecutesActualOwner() {
    for (long seed : List.of(0L, 117L, -1L, Long.MIN_VALUE)) {
      var scenario = ExtendedSemanticCorpus.generate(seed, 256); String script = ExtendedSemanticCorpus.write(scenario);
      assertEquals(scenario, ExtendedSemanticCorpus.parse(script)); assertEquals(scenario, ExtendedSemanticCorpus.parse(script.substring(0, script.length() - 1)));
      assertEquals(Set.of(Op.values()), scenario.steps().stream().map(Step::operation).collect(java.util.stream.Collectors.toSet()));
      assertEquals(script, ExtendedSemanticCorpus.write(ExtendedSemanticCorpus.generate(seed, 256)));
      try (var owner = UnifiedIMapExtendedSemanticAdapter.fresh("seeded-common-v2")) { new ExtendedSemanticOracle().compare(scenario, owner).requireEquivalent(); }
    }
    assertNotEquals(ExtendedSemanticCorpus.generate(0, 1), ExtendedSemanticCorpus.generate(1, 1));
    assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.generate(0, 0));
    assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.generate(0, 257));
  }

  @Test void binaryUnicodeEmptyAndOptionalPresenceAreUnambiguousAndTtlKindsSurvive() {
    var entries = List.of(new Entry(BytesValue.fromHex(""), Optional.of(BytesValue.fromHex("")), Optional.empty(), TtlDirective.eternal()));
    var scenario = new ExtendedSemanticScenario(2, -1, List.of(new Step("строка\n\t\\\"", Op.PUT, entries,
        List.of(new Request("клиент", 1)), Optional.empty(), 0, true)));
    String encoded = ExtendedSemanticCorpus.write(scenario); assertEquals(scenario, ExtendedSemanticCorpus.parse(encoded));
    assertTrue(encoded.contains("\t::~:e\t")); assertFalse(encoded.contains("клиент"));
    assertEquals(new ExtendedSemanticScenario(2, 0, List.of()), ExtendedSemanticCorpus.parse("imap-extended-semantic-v2\t0\t0"));
    var malformed = new ExtendedSemanticScenario(2, 0, List.of(new Step("bad\ud800", Op.RESOURCES, List.of(), List.of(), Optional.empty(), 0, false)));
    assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.write(malformed));
  }

  @Test void closedGrammarRefusesVersionFieldNumberHexUtf8AndPayloadAliasesBeforeExecution() {
    String prefix = "imap-extended-semantic-v2\t0\t1\n";
    String point = "61\tput\t:00:~:e\t63:1\t~\t0\t0\n";
    for (String invalid : List.of("", "imap-extended-semantic-v3\t0\t0", "imap-extended-semantic-v2\t-0\t0", "imap-extended-semantic-v2\t+1\t0", "imap-extended-semantic-v2\t9223372036854775808\t0",
        prefix, prefix + point + "\n", prefix + point.replace("\tput\t", "\tPUT\t"), prefix + point.replace("put", "future_operation"),
        prefix + point.replace(":00:", ":AA:"), prefix + point.replace(":00:", ":a:"), prefix + point.replace("63:1", "ff:1"), prefix + point.replace("63:1", "c080:1"),
        prefix + point.replace("63:1", "63:01"), prefix + point.replace("63:1", "63:0"), prefix + point.replace("~:e", "~:z"), prefix + point.replace("~:e", "~:x0"),
        prefix + point.replace("~:e", "~:x01"), prefix + point.replace("\t0\t0", "\t0\t2"), prefix + point.replace("\t~\t0", "\t1:1\t0"),
        prefix + point.replace("\t~\t0", "\t2147483648:1\t0"), prefix + point.replace("\tput", "\tput\textra"), prefix + point.replace("\t0\t0", "\t-1\t0"),
        prefix + point.replace("63:1", String.join(",", java.util.Collections.nCopies(65, "63:1"))), prefix + point.replace(":00:", ":" + "00".repeat(4097) + ":"),
        prefix + point.replace("63:1", "63:1:1"), prefix + point.replace(":00:~:e", ":00:~:e:extra"), prefix + point.replace("\n", "\r\n"), prefix + point.replace("63:1", "é:1"))) {
      assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.parse(invalid), invalid.substring(0, Math.min(100, invalid.length())));
    }
    assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.parse("x".repeat(ExtendedSemanticCorpus.MAX_BYTES + 1)));
    assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.parse("x".repeat(ExtendedSemanticCorpus.MAX_LINE_BYTES + 1)));
    assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.parse("\n".repeat(258)));
  }

  @Test void shrinkRetainsVersionSeedValidReferencesAndConsumesExplicitReplayAndWorkBounds() {
    var original = ExtendedSemanticCorpus.generate(117, 56);
    java.util.function.Predicate<ExtendedSemanticScenario> diverges = scenario -> {
      try (var owner = UnifiedIMapExtendedSemanticAdapter.fresh("shrink-canary", UnifiedIMapExtendedSemanticAdapter.Fault.DROP_EVENT)) {
        return !new ExtendedSemanticOracle().compare(scenario, owner).equivalent();
      } catch (IllegalArgumentException invalidReplay) { return false; }
    };
    var replayLimited = ExtendedSemanticCorpus.shrink(original, new SeededSemanticScenario.ShrinkBudget(1, 10000), diverges);
    assertTrue(replayLimited.budgetExhausted()); assertEquals(1, replayLimited.replays()); assertEquals(original.steps().size(), replayLimited.work());
    var workLimited = ExtendedSemanticCorpus.shrink(original, new SeededSemanticScenario.ShrinkBudget(100, original.steps().size()), diverges);
    assertTrue(workLimited.budgetExhausted()); assertEquals(1, workLimited.replays());
    var shrunk = ExtendedSemanticCorpus.shrink(original, new SeededSemanticScenario.ShrinkBudget(256, 100000), diverges);
    assertFalse(shrunk.budgetExhausted()); assertTrue(shrunk.scenario().steps().size() < original.steps().size()); assertTrue(diverges.test(shrunk.scenario()));
    assertEquals(117, shrunk.scenario().seed()); assertEquals(2, shrunk.scenario().version()); assertEquals(shrunk.scenario(), ExtendedSemanticCorpus.parse(ExtendedSemanticCorpus.write(shrunk.scenario())));
    assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.shrink(original, new SeededSemanticScenario.ShrinkBudget(1, 1), ignored -> true));
    var calls = new AtomicInteger(); assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticCorpus.shrink(original, new SeededSemanticScenario.ShrinkBudget(1, 1000), ignored -> { calls.incrementAndGet(); return false; })); assertEquals(1, calls.get());
    var empty = new ExtendedSemanticScenario(2, 1, List.of()); assertEquals(0, ExtendedSemanticCorpus.shrink(empty, new SeededSemanticScenario.ShrinkBudget(1, 1), ignored -> true).work());
    assertThrows(NullPointerException.class, () -> new ExtendedSemanticOracle().forEachExpected(original, null));
  }

  @Test void sharedSignedClockFixturesMatchIndependentExpectationsAndTheActualJavaOwner() throws Exception {
    for (String name : List.of("signed-point", "reserved-predeadline", "reserved-matching", "reserved-expired", "signed-bulk", "identifier-clients")) {
      try (var input = getClass().getResourceAsStream("/imap-v2-boundaries/" + name + ".tsv")) {
        assertNotNull(input); var bytes = input.readNBytes(ExtendedSemanticCorpus.MAX_BYTES + 1); assertTrue(bytes.length < ExtendedSemanticCorpus.MAX_BYTES);
        var scenario = ExtendedSemanticCorpus.parse(new String(bytes, java.nio.charset.StandardCharsets.US_ASCII));
        try (var owner = UnifiedIMapExtendedSemanticAdapter.fresh("shared-clock-" + name)) { new ExtendedSemanticOracle().compare(scenario, owner).requireEquivalent(); }
      }
    }
  }
}
