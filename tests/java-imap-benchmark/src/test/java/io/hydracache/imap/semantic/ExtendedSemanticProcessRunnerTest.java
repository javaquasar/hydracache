package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedSemanticScenario.*;
import static org.junit.jupiter.api.Assertions.*;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.OutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

final class ExtendedSemanticProcessRunnerTest {
  @TempDir Path directory;
  private static String run(String script, String... args) throws Exception {
    var bytes = new ByteArrayOutputStream();
    ExtendedSemanticProcessRunner.run(args, new ByteArrayInputStream(script.getBytes(StandardCharsets.UTF_8)), new PrintStream(bytes, true, StandardCharsets.UTF_8));
    return bytes.toString(StandardCharsets.UTF_8);
  }
  private static String body(String output) { return output.substring(output.indexOf('\n') + 1); }

  @Test void independentExpectedAndActualProcessFramesMatchAllActionsAndRealDefectsDiverge() throws Exception {
    String script = ExtendedSemanticCorpus.write(ExtendedSemanticCorpus.generate(117, 256));
    var generated = run("", "--v2-generate", "117", "256"); assertEquals(script, generated);
    String expected = run(script, "--v2-script", "-", "--v2-expected");
    String actual = run(script, "--v2-script", "-"); assertEquals(body(expected), body(actual));
    assertEquals(257, actual.lines().count()); assertTrue(expected.startsWith(ExtendedSemanticObservations.FORMAT + "\texpected\n"));
    for (var fault : UnifiedIMapExtendedSemanticAdapter.Fault.values()) {
      String output = run(script, "--v2-script", "-", "--v2-fault", fault.name().toLowerCase(java.util.Locale.ROOT));
      if (fault == UnifiedIMapExtendedSemanticAdapter.Fault.NONE) assertEquals(actual, output); else assertNotEquals(body(expected), body(output), fault.name());
    }
    Path file = directory.resolve("corpus.tsv"); Files.writeString(file, script); assertEquals(actual, run("", "--v2-script", file.toString()));
    var savedInput = System.in; var savedOutput = System.out; var output = new ByteArrayOutputStream();
    try { System.setIn(new ByteArrayInputStream(script.getBytes(StandardCharsets.UTF_8))); System.setOut(new PrintStream(output, true, StandardCharsets.UTF_8)); ExtendedSemanticProcessRunner.main(new String[] {"--v2-script", "-"}); }
    finally { System.setIn(savedInput); System.setOut(savedOutput); }
    assertEquals(actual, output.toString(StandardCharsets.UTF_8));
  }

  @Test void actualJvmSubprocessCanaryUsesTheSameCorpusAndChangesObservedOwnerEvents() throws Exception {
    Path script = directory.resolve("subprocess.tsv"); Files.writeString(script, ExtendedSemanticCorpus.write(ExtendedSemanticCorpus.generate(0, 4)));
    String java = Path.of(System.getProperty("java.home"), "bin", System.getProperty("os.name").startsWith("Windows") ? "java.exe" : "java").toString();
    var correct = new ProcessBuilder(java, "-cp", System.getProperty("java.class.path"), ExtendedSemanticProcessRunner.class.getName(), "--v2-script", script.toString()).redirectErrorStream(true).start();
    String correctOutput = new String(correct.getInputStream().readNBytes(65537), StandardCharsets.UTF_8); assertTrue(correct.waitFor(30, TimeUnit.SECONDS)); assertEquals(0, correct.exitValue());
    var canary = new ProcessBuilder(java, "-cp", System.getProperty("java.class.path"), ExtendedSemanticProcessRunner.class.getName(), "--v2-script", script.toString(), "--v2-fault", "drop_event").redirectErrorStream(true).start();
    String brokenOutput = new String(canary.getInputStream().readNBytes(65537), StandardCharsets.UTF_8); assertTrue(canary.waitFor(30, TimeUnit.SECONDS)); assertEquals(0, canary.exitValue());
    assertTrue(correctOutput.length() < 65536); assertTrue(correctOutput.contains("\"kind\":\"added\"")); assertFalse(brokenOutput.contains("\"kind\":\"added\"")); assertNotEquals(correctOutput, brokenOutput);
  }

  @Test void cliRefusesHostileArgumentsBytesAndOutputFailureWithoutUnboundedReads() throws Exception {
    String valid = ExtendedSemanticCorpus.write(ExtendedSemanticCorpus.generate(0, 1));
    for (String[] args : List.of(new String[0], new String[] {"--unknown", "-"}, new String[] {"--v2-script"},
        new String[] {"--v2-script", "-", "extra"}, new String[] {"--v2-script", "-", "--v2-fault", "DROP_EVENT"},
        new String[] {"--v2-script", "-", "--v2-fault", "future"}, new String[] {"--v2-generate", "0", "0"})) {
      assertThrows(IllegalArgumentException.class, () -> run(valid, args));
    }
    assertThrows(IllegalArgumentException.class, () -> run("x".repeat(ExtendedSemanticCorpus.MAX_BYTES + 1), "--v2-script", "-"));
    assertThrows(IllegalArgumentException.class, () -> run("é", "--v2-script", "-"));
    var failed = new PrintStream(new OutputStream() { @Override public void write(int value) throws java.io.IOException { throw new java.io.IOException("closed sink"); } });
    assertThrows(IllegalStateException.class, () -> ExtendedSemanticProcessRunner.run(new String[] {"--v2-script", "-"}, new ByteArrayInputStream(valid.getBytes(StandardCharsets.UTF_8)), failed));
    assertThrows(IllegalStateException.class, () -> ExtendedSemanticProcessRunner.run(new String[] {"--v2-generate", "0", "1"}, new ByteArrayInputStream(new byte[0]), failed));
    try (var sink = new PrintStream(OutputStream.nullOutputStream())) {
      var oversized = new ExtendedSemanticProcessRunner.Frames(sink, false);
      assertThrows(IllegalArgumentException.class, () -> oversized.line("x".repeat(ExtendedSemanticObservations.MAX_LINE_BYTES + 195)));
      var total = new ExtendedSemanticProcessRunner.Frames(sink, true);
      assertThrows(IllegalArgumentException.class, () -> { for (int i = 0; i < 17; i++) total.line("x".repeat(ExtendedSemanticObservations.MAX_LINE_BYTES)); });
    }
  }

  @Test void canonicalJsonIncludesPresenceTtlOriginalReceiptsRelationsCutAndEscapes() {
    var value = BytesValue.fromHex("");
    var item = new Item(0, value, "present", Optional.of(value), TtlState.eternal());
    var receipt = new Result("present", Optional.of(value), TtlState.expiring(2), List.of(item), List.of(), 9);
    var result = new Result("outcomes", Optional.empty(), TtlState.absent(), List.of(), List.of(
        new Status(new Request("к\"\\\n\t😀", 1), "completed", Optional.of(receipt)),
        new Status(new Request("missing", 2), "unknown", Optional.empty())), 2);
    var observed = new Observation(result, List.of(new Row(value, value, TtlState.eternal())),
        List.of(new Event("added", value, Optional.empty(), Optional.of(value), 1)), Map.of("b=a", false, "a=b", true), new Owners(1, 0, 1, true), 3);
    String json = ExtendedSemanticObservations.json(observed);
    assertTrue(json.contains("\"value\":null")); assertTrue(json.contains("\"value\":\"\"")); assertTrue(json.contains("\"number\":9"));
    assertTrue(json.contains("\"remaining_ticks\":2")); assertTrue(json.contains("\\\"\\\\\\u000a\\u0009😀"));
    assertTrue(json.contains("\"token_relations\":{\"a=b\":true,\"b=a\":false}")); assertTrue(json.endsWith("\"tick\":3}"));
    var heavyItems = java.util.Collections.nCopies(64, new Item(0, BytesValue.copyOf(new byte[1024]), "present", Optional.of(BytesValue.copyOf(new byte[4096])), TtlState.eternal()));
    var retained = new Result("bulk", Optional.empty(), TtlState.absent(), heavyItems, List.of(), 64);
    var heavy = new Result("outcomes", Optional.empty(), TtlState.absent(), List.of(), java.util.Collections.nCopies(64, new Status(new Request("c", 1), "completed", Optional.of(retained))), 64);
    assertThrows(IllegalArgumentException.class, () -> ExtendedSemanticObservations.json(new Observation(heavy, List.of(), List.of(), Map.of(), new Owners(0, 0, 0, false), 0)));
    for (String accepted : List.of("\u00a0", "\u0085", "\u2007", "\u202f")) assertDoesNotThrow(() -> ExtendedSemanticCorpus.parse(ExtendedSemanticCorpus.write(new ExtendedSemanticScenario(2, 0, List.of(new Step(accepted, Op.RESOURCES, List.of(), List.of(), Optional.empty(), 0, false))))));
    for (String blank : List.of("\u001c", "\u2003")) assertThrows(IllegalArgumentException.class, () -> new Request(blank, 1));
  }

  @Test void utf8FramesPreserveAllFourIdentifierClientsAcrossStreamAndJvmDefaultCharsets() throws Exception {
    byte[] fixture;
    try (var input = getClass().getResourceAsStream("/imap-v2-boundaries/identifier-clients.tsv")) { assertNotNull(input); fixture = input.readNBytes(65536); }
    String script = new String(fixture, StandardCharsets.US_ASCII);
    String expected = run(script, "--v2-script", "-", "--v2-expected");
    for (var charset : List.of(StandardCharsets.US_ASCII, StandardCharsets.ISO_8859_1)) for (boolean model : List.of(false, true)) {
      var raw = new ByteArrayOutputStream(); var output = new PrintStream(raw, true, charset);
      String[] args = model ? new String[] {"--v2-script", "-", "--v2-expected"} : new String[] {"--v2-script", "-"};
      ExtendedSemanticProcessRunner.run(args, new ByteArrayInputStream(fixture), output);
      String decoded = StandardCharsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(raw.toByteArray())).toString();
      assertEquals(body(expected), body(decoded));
      for (String client : List.of("\u00a0", "\u0085", "\u2007", "\u202f")) assertTrue(decoded.contains("\"client\":\"" + client + "\""));
    }
    var generated = new ByteArrayOutputStream();
    ExtendedSemanticProcessRunner.run(new String[] {"--v2-generate", "117", "1"}, new ByteArrayInputStream(new byte[0]), new PrintStream(generated, true, StandardCharsets.US_ASCII));
    assertEquals(ExtendedSemanticCorpus.generate(117, 1), ExtendedSemanticCorpus.parse(generated.toString(StandardCharsets.UTF_8)));
    Path path = directory.resolve("identifier-clients.tsv"); Files.write(path, fixture);
    String javaExecutable = Path.of(System.getProperty("java.home"), "bin", System.getProperty("os.name").startsWith("Windows") ? "java.exe" : "java").toString();
    for (boolean model : List.of(false, true)) {
      var arguments = new java.util.ArrayList<>(List.of(javaExecutable, "-Dfile.encoding=US-ASCII", "-cp", System.getProperty("java.class.path"), ExtendedSemanticProcessRunner.class.getName(), "--v2-script", path.toString()));
      if (model) arguments.add("--v2-expected");
      var child = new ProcessBuilder(arguments).redirectErrorStream(true).start();
      byte[] bytes = child.getInputStream().readNBytes(65537); assertTrue(child.waitFor(30, TimeUnit.SECONDS)); assertEquals(0, child.exitValue()); assertTrue(bytes.length < 65536);
      String decoded = StandardCharsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(bytes)).toString();
      assertEquals(body(expected), body(decoded));
    }
  }
}
