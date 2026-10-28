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
}
