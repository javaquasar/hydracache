package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedConcurrentScenario.*;
import static org.junit.jupiter.api.Assertions.*;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

final class ExtendedConcurrentTest {
  @TempDir Path directory;
  private static ExtendedConcurrentScenario one(String family, String order) { return new ExtendedConcurrentScenario(117, List.of(new Round(1, family, order))); }
  private static String run(String source, String... args) throws Exception {
    var output = new ByteArrayOutputStream(); ExtendedConcurrentRunner.run(args, new ByteArrayInputStream(source.getBytes(StandardCharsets.US_ASCII)), new PrintStream(output, true, StandardCharsets.US_ASCII)); return output.toString(StandardCharsets.UTF_8);
  }
  @Test void realTwoWorkerFamiliesMatchIndependentLegalHistoriesAcrossOrdersAndReplayCodec() {
    for (long seed : List.of(0L, 117L, -1L, Long.MIN_VALUE)) {
      var scenario = generate(seed, 15); assertEquals(scenario, parse(scenario.encode()));
      assertEquals(15, scenario.rounds().stream().map(round -> round.family() + round.order()).distinct().count());
      var histories = ExtendedConcurrentRunner.execute(scenario, false); assertEquals(histories, parseHistories(histories(seed, histories)).values());
      for (var history : histories) assertEquals(ExtendedConcurrentOracle.Verdict.VALID, new ExtendedConcurrentOracle().validate(history, 4096).verdict(), history.toString());
    }
  }
  @Test void independentCheckerAcceptsPerItemBulkInterleavingWithoutInventingTransactionAtomicity() {
    var round = new Round(1, "bulk_aba", "overlap");
    var history = new History(round, List.of(new Call(0, 0, 1, 10, "bulk:replaced,replaced"), new Call(1, 0, 2, 3, "removed"), new Call(1, 1, 4, 5, "inserted")),
        "61/00/e/0,62/02/e/0|updated/61/00/01/3,removed/61/01/~/4,added/61/~/00/5,updated/62/00/02/6||5/0/0/0/0");
    assertEquals(ExtendedConcurrentOracle.Verdict.VALID, new ExtendedConcurrentOracle().validate(history, 4096).verdict());
    assertEquals(ExtendedConcurrentOracle.Verdict.SEARCH_EXHAUSTED, new ExtendedConcurrentOracle().validate(history, 1).verdict());
    var fabricated = new History(round, history.calls(), history.finalState().replace("5/0/0/0/0", "5/1/0/0/0"));
    assertEquals(ExtendedConcurrentOracle.Verdict.INVALID, new ExtendedConcurrentOracle().validate(fabricated, 4096).verdict());
    for (int budget : List.of(0, -1, 100001)) assertThrows(IllegalArgumentException.class, () -> new ExtendedConcurrentOracle().validate(history, budget));
  }
  @Test void actualBrokenOwnerAndImpossibleCutRetirementAndCheckpointClaimsAreRejected() {
    var broken = ExtendedConcurrentRunner.execute(one("bulk_aba", "right_first"), true).get(0);
    assertEquals(ExtendedConcurrentOracle.Verdict.INVALID, new ExtendedConcurrentOracle().validate(broken, 4096).verdict());
    for (var fixture : List.of(one("listener_cut", "right_first"), one("retire_pending", "left_first"), one("checkpoint_lease", "right_first"))) {
      var actual = ExtendedConcurrentRunner.execute(fixture, false).get(0); var calls = new ArrayList<>(actual.calls());
      if (actual.round().family().equals("listener_cut")) {
        var corrupted = new History(actual.round(), calls, actual.finalState().replace("added/63/~/03/3", "added/62/~/00/2"));
        assertEquals(ExtendedConcurrentOracle.Verdict.INVALID, new ExtendedConcurrentOracle().validate(corrupted, 4096).verdict());
      } else {
        int at = actual.round().family().equals("retire_pending") ? 3 : 1; var old = calls.get(at);
        calls.set(at, new Call(old.worker(), old.ordinal(), old.start(), old.end(), actual.round().family().equals("retire_pending") ? "retired:0" : "checkpoint"));
        assertEquals(ExtendedConcurrentOracle.Verdict.INVALID, new ExtendedConcurrentOracle().validate(new History(actual.round(), calls, actual.finalState()), 4096).verdict());
      }
    }
    var observer = new ExtendedConcurrentRunner.Subscriber(); var failure = new IllegalStateException("listener failure"); observer.onError(failure); assertSame(failure, observer.failure.get()); observer.onComplete(); assertTrue(observer.failure.get().getMessage().contains("completed")); assertTrue(observer.delivered.isEmpty());
  }
  @Test void workerFailureRefusesToPublishAnApparentlyLegalPartialHistory() {
    var failure = new IllegalStateException("worker boundary fault");
    var propagated = assertThrows(IllegalStateException.class, () -> ExtendedConcurrentRunner.execute(one("checkpoint_lease", "left_first"), false, (worker, ordinal) -> { if (worker == 0) throw failure; }));
    assertTrue(propagated.getMessage().contains("worker failed"));
    assertSame(failure, propagated.getCause().getCause());
    assertEquals(ExtendedConcurrentOracle.Verdict.VALID, new ExtendedConcurrentOracle().validate(ExtendedConcurrentRunner.execute(one("checkpoint_lease", "right_first"), false).get(0), 4096).verdict());
  }
  @Test void checkerRefusesSerializedHistoryRelabelledAsConcurrentOverlap() throws Exception {
    for (String order : List.of("left_first", "right_first")) {
      var serial = ExtendedConcurrentRunner.execute(one("bulk_aba", order), false).get(0);
      String forged = histories(117, List.of(serial)).replace("\t" + order + "\t", "\toverlap\t");
      assertTrue(assertThrows(IllegalArgumentException.class, () -> parseHistories(forged)).getMessage().contains("overlap"));
      assertThrows(IllegalArgumentException.class, () -> run(forged, "--check-history", "-"));
    }
    var actual = ExtendedConcurrentRunner.execute(one("bulk_aba", "overlap"), false).get(0);
    assertTrue(actual.calls().get(0).start() < actual.calls().get(1).end());
    assertTrue(actual.calls().get(1).start() < actual.calls().get(0).end());
    assertEquals(ExtendedConcurrentOracle.Verdict.VALID, new ExtendedConcurrentOracle().validate(actual, 4096).verdict());
  }
  @Test void closedScheduleAndHistoryCodecRefuseUnknownAmbiguousOversizedAndImpossibleIntervals() {
    String valid = one("bulk_aba", "overlap").encode();
    for (String invalid : List.of("", "imap-extended-schedule-v2\t0\n1\tbulk_aba\toverlap\n", valid.replace("117", "-0"), valid.replace("117", "9223372036854775808"), valid.replace("1\tbulk", "0\tbulk"), valid.replace("bulk_aba", "future"), valid.replace("overlap", "OVERLAP"), valid + "1\tbulk_aba\toverlap\n", valid.replace("\n", "\r\n"), valid.replace("117", "é"), "x".repeat(MAX_SCHEDULE_BYTES + 1))) assertThrows(IllegalArgumentException.class, () -> parse(invalid));
    for (int count : List.of(0, 33)) assertThrows(IllegalArgumentException.class, () -> generate(0, count));
    assertThrows(IllegalArgumentException.class, () -> new Round(33, "bulk_aba", "overlap")); assertThrows(IllegalArgumentException.class, () -> new Round(1, "bulk_aba", "overlap").program(2));
    var history = ExtendedConcurrentRunner.execute(one("bulk_aba", "left_first"), false).get(0); String encoded = histories(117, List.of(history));
    for (String invalid : List.of("", encoded.replace(HISTORY, "future"), encoded.replace("\t1\n", "\t2\n"), encoded.replace(".1.", ".0."), encoded.replace("\n", "\r\n"), encoded.replace("bulk_aba", "future"), encoded.replace("117", "-0"), encoded + "extra\n", "x".repeat(MAX_HISTORY_BYTES + 1))) assertThrows(IllegalArgumentException.class, () -> parseHistories(invalid));
    assertThrows(IllegalArgumentException.class, () -> new Call(0, 0, 1, 2, "é"));
    assertThrows(IllegalArgumentException.class, () -> new Call(2, 0, 1, 2, "ok"));
    assertThrows(IllegalArgumentException.class, () -> new Call(0, 3, 1, 2, "ok"));
    assertThrows(IllegalArgumentException.class, () -> new Call(0, 0, 2, 1, "ok"));
    assertThrows(IllegalArgumentException.class, () -> new History(history.round(), List.of(), ""));
    assertThrows(IllegalArgumentException.class, () -> new History(history.round(), history.calls(), "x".repeat(8193)));
    assertThrows(IllegalArgumentException.class, () -> new ExtendedConcurrentScenario(0, List.of()));
    assertThrows(IllegalArgumentException.class, () -> histories(0, List.of()));
    assertThrows(IllegalArgumentException.class, () -> new History(history.round(), List.of(history.calls().get(0), history.calls().get(0), history.calls().get(2)), ""));
    var overlap = ExtendedConcurrentRunner.execute(one("bulk_aba", "overlap"), false).get(0);
    assertThrows(IllegalArgumentException.class, () -> new History(new Round(1, "bulk_aba", "left_first"), overlap.calls(), overlap.finalState()));
    assertThrows(IllegalArgumentException.class, () -> parse(valid.repeat(34)));
    var fields = encoded.split("\n")[1].split("\t"); String prefix = HISTORY + "\t117\t1\n1\tbulk_aba\tleft_first\t";
    for (String calls : List.of("0.0.1.2.ff,1.0.3.4.00,1.1.5.6.00", "0.0.1.2.0A,1.0.3.4.00,1.1.5.6.00", "0.0.1.2.f,1.0.3.4.00,1.1.5.6.00", fields[3] + ",0.0.1.2.,0.0.1.2.,0.0.1.2.")) assertThrows(IllegalArgumentException.class, () -> parseHistories(prefix + calls + "\t\n"));
    assertThrows(IllegalArgumentException.class, () -> parseHistories(encoded.replace("0.0.", "65.0.")));
    var oversized = new ArrayList<History>();
    for (int index = 1; index <= 32; index++) oversized.add(new History(new Round(index, "bulk_aba", "left_first"), history.calls(), "x".repeat(8192)));
    assertThrows(IllegalArgumentException.class, () -> histories(117, oversized));
  }
  @Test void boundedCliRunsActualWorkersChecksHistoriesAndRealJvmDefectCannotEscape() throws Exception {
    var scenario = generate(117, 15); assertEquals(scenario.encode(), run("", "--generate", "117", "15"));
    String actual = run(scenario.encode(), "--schedule", "-"); String verdicts = run(actual, "--check-history", "-", "--search-budget", "4096");
    assertEquals(15, verdicts.lines().filter(line -> line.contains("\tvalid\t")).count()); assertTrue(run(actual, "--check-history", "-", "--search-budget", "1").contains("search_exhausted"));
    String broken = run(one("bulk_aba", "right_first").encode(), "--schedule", "-", "--canary-unconditional-version"); assertTrue(run(broken, "--check-history", "-").contains("\tinvalid\t"));
    Path script = directory.resolve("extended-schedule.tsv"); Files.writeString(script, one("bulk_aba", "right_first").encode()); assertFalse(run("", "--schedule", script.toString()).isEmpty());
    String executable = Path.of(System.getProperty("java.home"), "bin", System.getProperty("os.name").startsWith("Windows") ? "java.exe" : "java").toString();
    var child = new ProcessBuilder(executable, "-cp", System.getProperty("java.class.path"), ExtendedConcurrentRunner.class.getName(), "--schedule", script.toString(), "--canary-unconditional-version").redirectErrorStream(true).start();
    String bad = new String(child.getInputStream().readNBytes(MAX_HISTORY_BYTES + 1), StandardCharsets.US_ASCII); assertTrue(child.waitFor(10, java.util.concurrent.TimeUnit.SECONDS)); assertEquals(0, child.exitValue()); assertTrue(run(bad, "--check-history", "-").contains("\tinvalid\t"));
    var savedIn = System.in; var savedOut = System.out; var output = new ByteArrayOutputStream(); try { System.setIn(new ByteArrayInputStream(scenario.encode().getBytes(StandardCharsets.US_ASCII))); System.setOut(new PrintStream(output, true, StandardCharsets.US_ASCII)); ExtendedConcurrentRunner.main(new String[] {"--schedule", "-"}); } finally { System.setIn(savedIn); System.setOut(savedOut); } assertEquals(15, parseHistories(output.toString(StandardCharsets.US_ASCII)).values().size());
    for (String[] args : List.of(new String[0], new String[] {"bad", "-"}, new String[] {"--schedule", "-", "future"}, new String[] {"--check-history", "-", "--search-budget", "0"})) assertThrows(IllegalArgumentException.class, () -> run(actual, args));
    assertThrows(IllegalArgumentException.class, () -> run("x".repeat(MAX_HISTORY_BYTES + 1), "--check-history", "-"));
    assertThrows(IllegalArgumentException.class, () -> ExtendedConcurrentRunner.run(new String[] {"--schedule", "-"}, new ByteArrayInputStream(new byte[] {(byte) 255}), new PrintStream(new ByteArrayOutputStream())));
    var denied = new PrintStream(new java.io.OutputStream() { public void write(int ignored) throws java.io.IOException { throw new java.io.IOException("denied"); } });
    assertThrows(IllegalStateException.class, () -> ExtendedConcurrentRunner.run(new String[] {"--generate", "0", "1"}, InputStreamNull(), denied));
  }
  private static java.io.InputStream InputStreamNull() { return new ByteArrayInputStream(new byte[0]); }
}
