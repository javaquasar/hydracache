package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.StringReader;
import org.junit.jupiter.api.Test;

final class ScenarioManifestLoaderTest {
  private static final ManifestLimits LIMITS = new ManifestLimits(4096, 8, 16, 32, 4, 128, 256);

  @Test
  void parsesTheFrozenLineFormatDeterministically() throws Exception {
    String manifest = """
        format=imap-semantic-v1
        seed=42
        step=s1 put key=6b31 value=7631 ttl=eternal
        step=s2 get key=6b31
        step=s3 put_all entries=6b32:7632,6b33:7633 ttl=expire_after:5
        step=s4 advance ticks=5
        step=s5 get_all keys=6b31,6b32,6b33
        """;

    ScenarioManifest parsed = ScenarioManifestLoader.load(new StringReader(manifest), LIMITS);

    assertEquals(42, parsed.seed());
    assertEquals(5, parsed.steps().size());
    assertEquals(Operation.PUT, parsed.steps().get(0).operation());
    assertEquals(TtlDirective.eternal(), parsed.steps().get(0).ttl());
    assertEquals(Operation.ADVANCE, parsed.steps().get(3).operation());
    assertEquals(5, parsed.steps().get(3).advanceTicks());
  }

  @Test
  void rejectsUnknownFieldsAndOperations() {
    assertThrows(ManifestException.class, () -> ScenarioManifestLoader.load(
        new StringReader("format=imap-semantic-v1\nseed=1\nstep=s1 get key=00 surprise=yes\n"),
        LIMITS));
    assertThrows(ManifestException.class, () -> ScenarioManifestLoader.load(
        new StringReader("format=imap-semantic-v1\nseed=1\nstep=s1 execute_on_key key=00\n"),
        LIMITS));
  }

  @Test
  void rejectsInputBeforeUnboundedAllocation() {
    String tooManyItems = "format=imap-semantic-v1\nseed=1\n"
        + "step=s1 get_all keys=00,01,02,03,04\n";
    String oversizedValue = "format=imap-semantic-v1\nseed=1\n"
        + "step=s1 put key=00 value=" + "aa".repeat(33) + " ttl=eternal\n";

    assertThrows(ManifestException.class,
        () -> ScenarioManifestLoader.load(new StringReader(tooManyItems), LIMITS));
    assertThrows(ManifestException.class,
        () -> ScenarioManifestLoader.load(new StringReader(oversizedValue), LIMITS));
  }
}
