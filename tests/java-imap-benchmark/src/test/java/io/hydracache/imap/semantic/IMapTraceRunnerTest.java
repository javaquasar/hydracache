package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.StringReader;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import org.junit.jupiter.api.Test;

final class IMapTraceRunnerTest {
  private static String step(String operation, String key, String value, String expected,
      String ttl, int identity, String detail, String extra) {
    return String.join("\t", operation, key, value, expected, ttl,
        Integer.toString(identity), "0", "1000", detail, extra) + "\n";
  }

  @Test
  void processPeerExecutesRealServiceAndItsDefectChangesState() throws Exception {
    String source = "imap-api-trace-v1\t117\n"
        + step("entry_listener", "", "~", "~", "eternal", 1, "values", "")
        + step("set", "61", "31", "~", "eternal", 2, "", "")
        + step("get_entry_view", "61", "~", "~", "eternal", 3, "", "")
        + step("replace_if_version", "61", "32", "~", "preserve", 4, "remembered", "")
        + step("remove_if_version", "61", "~", "~", "eternal", 5, "stale", "")
        + step("get", "61", "~", "~", "eternal", 6, "", "")
        + step("get_all", "", "~", "~", "eternal", 7, "", "61,62")
        + step("put_all_detailed", "", "~", "~", "eternal", 8, "", "62:31,63:32")
        + step("remove_entry_listener", "", "~", "~", "eternal", 1, "", "")
        + step("unsupported", "", "~", "~", "eternal", 0, "", "");
    List<String> correct = IMapTraceRunner.execute(new StringReader(source), false);
    List<String> divergent = IMapTraceRunner.execute(new StringReader(source), true);
    assertEquals(10, correct.size());
    assertNotEquals(correct, divergent);
    assertTrue(correct.get(1).contains("added/61/"));
    assertTrue(correct.get(5).startsWith("present|32|"));
    assertTrue(correct.get(9).startsWith("error:unsupported_operation|"));
  }

  @Test
  void runnerRejectsMalformedAndOversizedDevelopmentFrames() {
    for (String invalid : List.of("", "imap-api-trace-v2\t1\n", "imap-api-trace-v1\t1\n",
        "imap-api-trace-v1\t-1\n", "imap-api-trace-v1\t1\r\n")) {
      assertThrows(IllegalArgumentException.class,
          () -> IMapTraceRunner.execute(new StringReader(invalid), false));
    }
    for (String invalid : List.of(
        step("GET", "", "~", "~", "eternal", 0, "", ""),
        step("get", "0", "~", "~", "eternal", 0, "", ""),
        step("get", "zz", "~", "~", "eternal", 0, "", ""),
        step("get", "", "~", "~", "after:0", 0, "", ""),
        step("get", "", "~", "~", "unknown", 0, "", ""),
        step("key_set_page", "", "~", "~", "eternal", 0, "2147483648", ""),
        step("get", "", "~", "~", "eternal", 0, ";", ""))) {
      assertThrows(IllegalArgumentException.class,
          () -> IMapTraceRunner.execute(new StringReader("imap-api-trace-v1\t1\n" + invalid), false));
    }
    assertThrows(IllegalArgumentException.class,
        () -> IMapTraceRunner.execute(new StringReader("x".repeat(8193)), false));
    assertThrows(IllegalArgumentException.class,
        () -> IMapTraceRunner.execute(new StringReader("é"), false));
    assertThrows(IllegalArgumentException.class,
        () -> IMapTraceRunner.execute(new StringReader(("x".repeat(8192) + "\n").repeat(65)), false));
    assertThrows(IllegalArgumentException.class, () -> IMapTraceRunner.execute(new StringReader(
        "imap-api-trace-v1\t1\n" + step("get", "", "~", "~", "eternal", 0, "", "").repeat(513)), false));
  }

  @Test
  void replacementBulkTraceUsesInputOrderAndChangesObservedValues() throws Exception {
    String source = "imap-api-trace-v1\t117\n"
        + step("entry_listener", "", "~", "~", "eternal", 1, "values", "")
        + step("put_all_detailed", "", "~", "~", "eternal", 2, "", "62:31,61:32")
        + step("replace_all_detailed", "", "~", "~", "eternal", 3, "", "62:33,61:34")
        + step("get_all", "", "~", "~", "eternal", 0, "", "62,61");
    var results = IMapTraceRunner.execute(new StringReader(source), false);
    String replacement = results.get(2);
    assertTrue(replacement.startsWith("bulk|"));
    assertTrue(replacement.indexOf("/updated/62/") >= 0);
    assertTrue(replacement.indexOf("/updated/62/") < replacement.indexOf("/updated/61/"));
    assertTrue(results.get(3).contains("0/62/present/33/"));
    assertTrue(results.get(3).contains("1/61/present/34/"));
  }

  @Test
  void mainExecutesBoundedStdioAndRejectsUnknownCanaryArguments() throws Exception {
    var oldIn = System.in;
    var oldOut = System.out;
    String source = "imap-api-trace-v1\t117\n"
        + step("set", "61", "31", "~", "after:3", 1, "", "")
        + step("get", "61", "~", "~", "eternal", 0, "", "");
    try {
      var correct = new ByteArrayOutputStream();
      System.setIn(new ByteArrayInputStream(source.getBytes(StandardCharsets.US_ASCII)));
      System.setOut(new PrintStream(correct, true, StandardCharsets.US_ASCII));
      IMapTraceRunner.main(new String[0]);
      var divergent = new ByteArrayOutputStream();
      System.setIn(new ByteArrayInputStream(source.getBytes(StandardCharsets.US_ASCII)));
      System.setOut(new PrintStream(divergent, true, StandardCharsets.US_ASCII));
      IMapTraceRunner.main(new String[] {"--canary-drop-set"});
      assertNotEquals(correct.toString(StandardCharsets.US_ASCII),
          divergent.toString(StandardCharsets.US_ASCII));
      assertThrows(IllegalArgumentException.class,
          () -> IMapTraceRunner.main(new String[] {"--unknown"}));
    } finally {
      System.setIn(oldIn);
      System.setOut(oldOut);
    }
  }

  @Test
  void tracesCoverCursorContinuationPendingRecoveryAndGenerationMismatches() throws Exception {
    String source = "imap-api-trace-v1\t117\n"
        + step("set", "61", "31", "~", "eternal", 1, "", "")
        + step("set", "62", "32", "~", "eternal", 2, "", "")
        + step("set", "63", "33", "~", "eternal", 3, "", "")
        + step("key_set_page", "", "~", "~", "eternal", 0, "1", "")
        + step("key_set_page", "", "~", "~", "eternal", 0, "resume", "")
        + step("values_page", "", "~", "~", "eternal", 0, "wrong_cursor", "")
        + step("get_entry_view", "61", "~", "~", "eternal", 0, "", "")
        + step("replace_if_version", "61", "34", "~", "preserve", 4, "wrong_generation", "")
        + step("reserve_operation", "64", "35", "~", "eternal", 5, "", "")
        + step("query_operation_outcome", "", "~", "~", "eternal", 5, "", "")
        + step("cancel_reserved", "", "~", "~", "eternal", 5, "", "")
        + step("query_operation_outcome", "", "~", "~", "eternal", 5, "", "");
    var results = IMapTraceRunner.execute(new StringReader(source.substring(0, source.length() - 1)), false);
    assertTrue(results.get(3).startsWith("partial|"));
    assertTrue(results.get(4).startsWith("partial|"));
    assertTrue(results.get(5).startsWith("error:"));
    assertTrue(results.get(7).startsWith("mismatch|"));
    assertTrue(results.get(9).startsWith("pending|"));
    assertTrue(results.get(11).startsWith("unknown|"));
    assertThrows(IllegalArgumentException.class, () -> IMapTraceRunner.execute(new StringReader(
        "imap-api-trace-v1\t1\n" + step("put_all_detailed", "", "~", "~", "eternal", 1, "", "61")), false));
  }

  @Test
  void typedLifecycleFramesExecuteRealBudgetsRetirementResyncVersionBatchesAndRestore() throws Exception {
    String source = "imap-api-trace-v1\t117\n"
        + step("reset_limits", "", "~", "~", "eternal", 0, "96", "1")
        + step("entry_listener", "", "~", "~", "eternal", 10000, "values", "1")
        + step("put", "78", "31", "~", "after:5", 10001, "", "")
        + step("put", "79", "32", "~", "after:5", 10002, "", "")
        + step("put", "7a", "01".repeat(100), "~", "eternal", 10003, "", "")
        + step("reserve_operation", "70", "70", "~", "eternal", 10004, "", "")
        + step("retire_outcomes", "", "~", "~", "eternal", 10004, "", "")
        + step("query_outcomes", "", "~", "~", "eternal", 0, "4096", "10001,10004,99999")
        + step("cancel_reserved", "", "~", "~", "eternal", 10004, "", "")
        + step("retire_outcomes", "", "~", "~", "eternal", 10002, "", "")
        + step("put", "78", "31", "~", "after:5", 10001, "", "")
        + step("query_outcomes", "", "~", "~", "eternal", 0, "4096", "10001,10004,99999")
        + step("get_entry_view", "78", "~", "~", "eternal", 0, "", "")
        + step("get_entry_view", "79", "~", "~", "eternal", 0, "", "")
        + step("version_bulk_replace", "", "~", "~", "preserve", 10005, "", "79:33,78:34")
        + step("version_bulk_replace", "", "~", "~", "preserve", 10006, "", "79:33,78:34")
        + step("version_bulk_replace", "", "~", "~", "preserve", 10005, "", "79:33,78:34")
        + step("resync_listener", "", "~", "~", "eternal", 10000, "2", "4096")
        + step("get_entry_view", "78", "~", "~", "eternal", 0, "", "")
        + step("get_entry_view", "79", "~", "~", "eternal", 0, "", "")
        + step("version_bulk_set_ttl", "", "~", "~", "after:1", 10007, "", "79,78")
        + step("get_entry_view", "78", "~", "~", "eternal", 0, "", "")
        + step("get_entry_view", "79", "~", "~", "eternal", 0, "", "")
        + step("version_bulk_remove", "", "~", "~", "eternal", 10008, "", "78")
        + step("checkpoint_restore", "", "~", "~", "eternal", 0, "", "")
        + step("remove_entry_listener", "", "~", "~", "eternal", 10000, "", "")
        + step("checkpoint_restore", "", "~", "~", "eternal", 0, "", "")
        + step("version_bulk_remove", "", "~", "~", "eternal", 10009, "", "79")
        + step("put", "78", "31", "~", "after:5", 10001, "", "")
        + step("resource_usage", "", "~", "~", "eternal", 0, "", "");
    var results = IMapTraceRunner.execute(new StringReader(source), false);
    assertTrue(results.get(4).startsWith("error:resource_budget_exhausted|"));
    assertTrue(results.get(6).startsWith("error:pending_retirement|"));
    assertTrue(results.get(7).contains("/completed:inserted/"));
    assertTrue(results.get(7).contains("/pending/"));
    assertTrue(results.get(10).startsWith("error:retired_identity|"));
    assertTrue(results.get(11).contains("/unknown/"));
    assertTrue(results.get(14).contains("0/79/replaced/"));
    assertTrue(results.get(15).contains("0/79/mismatch/"));
    assertEquals(List.of(results.get(14).split("\\|", -1)).subList(0, 7), List.of(results.get(16).split("\\|", -1)).subList(0, 7));
    assertTrue(results.get(17).startsWith("resynced|"));
    assertTrue(results.get(17).contains("/78/present/34/"));
    assertTrue(results.get(24).startsWith("error:active_owners|"));
    assertTrue(results.get(26).startsWith("restored|"));
    assertTrue(results.get(27).contains("0/79/mismatch/"));
    assertTrue(results.get(28).startsWith("error:retired_identity|"));
    assertTrue(results.get(29).startsWith("resources|"));
    assertThrows(IllegalArgumentException.class, () -> IMapTraceRunner.execute(new StringReader(
        "imap-api-trace-v1\t1\n" + step("version_bulk_replace", "", "~", "~", "eternal", 1, "", "61")), false));
    var absent = IMapTraceRunner.execute(new StringReader("imap-api-trace-v1\t1\n"
        + step("version_bulk_remove", "", "~", "~", "eternal", 1, "", "61")), false);
    assertTrue(absent.get(0).contains("0/61/absent/"));
  }

  @Test
  void readOnlyTraceObservationDoesNotConsumeIncrementalExpiryWork() throws Exception {
    String source = "imap-api-trace-v1\t117\n"
        + step("reset_limits", "", "~", "~", "eternal", 0, "96", "1")
        + step("entry_listener", "", "~", "~", "eternal", 1, "values", "")
        + step("set", "78", "65", "~", "after:1", 2, "", "")
        + step("set", "79", "65", "~", "after:1", 3, "", "")
        + step("set", "7a", "65", "~", "after:1", 4, "", "");
    String expire = step("expire_step", "", "~", "~", "eternal", 0, "1", "")
        .replace("\t0\t1000\t", "\t1\t1000\t");
    var results = IMapTraceRunner.execute(new StringReader(source + expire.repeat(4)), false);
    for (int index = 0; index < 4; index++) {
      String[] fields = results.get(index + 5).split("\\|", -1);
      assertEquals("expiry_progress", fields[0]);
      assertEquals("[]", fields[7]);
      assertEquals(index < 2 ? "1" : "0", fields[5]);
      assertTrue(fields[10].startsWith("[" + new int[] {52, 26, 0, 0}[index] + ","));
      assertEquals(index < 3 ? 1 : 0, fields[8].split("/expired/", -1).length - 1);
    }
  }
}
