package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.StringReader;
import java.util.EnumSet;
import org.junit.jupiter.api.Test;

final class SemanticOracleTest {
  private static final ManifestLimits LIMITS = ManifestLimits.defaults();

  @Test
  void independentAdaptersProduceTheSameOutcomesEventsAndFinalDigest() throws Exception {
    ScenarioManifest scenario = load("""
        format=imap-semantic-v1
        seed=20261004
        step=s1 put key=61 value=31 ttl=eternal
        step=s2 put_if_absent key=61 value=32 ttl=preserve
        step=s3 replace key=61 expected=31 value=33 ttl=expire_after:3
        step=s4 get_and_put key=62 value=34 ttl=eternal
        step=s5 get_all keys=61,62,63
        step=s6 advance ticks=3
        step=s7 get key=61
        step=s8 remove_all keys=61,62,63
        """);

    try (MapSemanticAdapter left = InMemorySemanticAdapter.strict("left");
         MapSemanticAdapter right = InMemorySemanticAdapter.strict("right")) {
      OracleReport report = new SemanticOracle().compare(scenario, left, right);

      assertTrue(report.equivalent(), report::render);
      assertEquals(0, report.differences().size());
      assertEquals(report.leftSnapshot().digest(), report.rightSnapshot().digest());
      assertEquals(report.leftSnapshot().liveCardinality(), report.rightSnapshot().liveCardinality());
    }
  }

  @Test
  void detectsReturnValueAndFinalStateDivergenceBeforePerformanceMayRun() throws Exception {
    ScenarioManifest scenario = load("""
        format=imap-semantic-v1
        seed=7
        step=s1 put key=61 value=31 ttl=eternal
        step=s2 get_and_put key=61 value=32 ttl=eternal
        step=s3 get key=61
        """);

    try (MapSemanticAdapter correct = InMemorySemanticAdapter.strict("correct");
         MapSemanticAdapter divergent = InMemorySemanticAdapter.withDefect(
             "divergent", InMemorySemanticAdapter.Defect.DROP_GET_AND_PUT)) {
      OracleReport report = new SemanticOracle().compare(scenario, correct, divergent);

      assertFalse(report.equivalent());
      assertTrue(report.differences().stream()
          .anyMatch(difference -> difference.path().contains("s2")
              || difference.path().equals("final.snapshot")), report::render);
      assertThrows(SemanticMismatchException.class, report::requireEquivalent);
    }
  }

  @Test
  void detectsUnsupportedCapabilityBeforeExecutingTheTrace() throws Exception {
    ScenarioManifest scenario = load("""
        format=imap-semantic-v1
        seed=8
        step=s1 put_if_absent key=61 value=31 ttl=eternal
        """);
    MapSemanticAdapter limited = InMemorySemanticAdapter.withCapabilities(
        "limited", EnumSet.of(Operation.GET, Operation.PUT));

    try (limited; MapSemanticAdapter complete = InMemorySemanticAdapter.strict("complete")) {
      OracleReport report = new SemanticOracle().compare(scenario, limited, complete);
      assertFalse(report.equivalent());
      assertTrue(report.differences().get(0).path().startsWith("capability."), report::render);
    }
  }

  private static ScenarioManifest load(String text) throws Exception {
    return ScenarioManifestLoader.load(new StringReader(text), LIMITS);
  }
}
