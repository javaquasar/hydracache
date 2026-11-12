package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.io.StringReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;

final class IMapConcurrentRunnerTest {
  private static String schedule(String... rows) {
    return "imap-api-schedule-v1\t117\n" + String.join("\n", rows) + "\n";
  }

  @Test
  void actualWorkersHonorExplicitOverlapAndRealTimeSchedules() throws Exception {
    var rows = new ArrayList<String>();
    for (String kind : List.of("compare", "duplicate", "expiry")) {
      for (String order : List.of("overlap", "left_first", "right_first")) {
        rows.add((rows.size() + 1) + "\t" + kind + "\t" + order + "\t17\t18");
      }
    }
    var histories = IMapConcurrentRunner.execute(new StringReader(schedule(rows.toArray(String[]::new))), false);
    assertEquals(9, histories.size());
    for (int index = 0; index < histories.size(); index++) {
      String[] fields = histories.get(index).split("\t", -1);
      assertEquals(15, fields.length);
      long beginLeft = Long.parseLong(fields[3]), endLeft = Long.parseLong(fields[4]);
      long beginRight = Long.parseLong(fields[6]), endRight = Long.parseLong(fields[7]);
      assertTrue(beginLeft < endLeft && beginRight < endRight);
      switch (fields[2]) {
        case "overlap" -> assertTrue(Math.max(beginLeft, beginRight) < Math.min(endLeft, endRight));
        case "left_first" -> assertTrue(endLeft < beginRight);
        case "right_first" -> assertTrue(endRight < beginLeft);
        default -> throw new AssertionError("unknown fixture order");
      }
      if (fields[1].equals("compare")) {
        assertTrue(fields[5].startsWith("replaced|") ^ fields[8].startsWith("replaced|"));
        assertTrue(fields[12].startsWith("completed|"));
        assertEquals("[1,1,3,0,1,0]", fields[14]);
      } else if (fields[1].equals("duplicate")) {
        assertEquals(fields[5], fields[8]);
        assertEquals(fields[5], fields[11]);
        assertEquals("[updated/6b/00/11/3/2]", fields[10]);
        assertEquals("[1,1,2,0,1,0]", fields[14]);
      } else {
        assertEquals("absent|~|~|~|0|0|[]", fields[5]);
        assertEquals("[]", fields[9]);
        assertEquals("[expired/6b/00/~/3/2]", fields[10]);
      }
    }
  }

  @Test
  void realAsyncFacadeCancellationDistinguishesAdmissionAndRetainsAppliedWork() throws Exception {
    var histories = IMapConcurrentRunner.execute(new StringReader(schedule(
        "1\tcancel_before\tleft_first\t1\t2", "2\tcancel_after\tleft_first\t1\t2")), false);
    String[] before = histories.get(0).split("\t", -1), after = histories.get(1).split("\t", -1);
    assertEquals("not_applied|~|~|~|0|0|[]", before[5]);
    assertEquals("[6b/00/2/10]", before[9]);
    assertEquals("unknown|~|~|~|0|0|[]", before[12]);
    assertEquals("cancelled_before/not_applied/unknown", before[13]);
    assertEquals("outcome_unknown|~|~|~|0|0|[]", after[5]);
    assertEquals("applied|~|3|10|0|1|[]", after[8]);
    assertEquals(after[8], after[11]);
    assertEquals("[6b/01/3/10]", after[9]);
    assertEquals("cancelled_after/outcome_unknown/pending", after[13]);
    assertEquals("[1,1,2,0,1,0]", after[14]);
  }

  @Test
  void canaryExecutesAnActualBrokenConditionalWrite() throws Exception {
    String source = schedule("1\tcompare\tleft_first\t1\t2");
    var correct = IMapConcurrentRunner.execute(new StringReader(source), false);
    var defective = IMapConcurrentRunner.execute(new StringReader(source), true);
    assertNotEquals(correct, defective);
    assertEquals("[6b/01/3/10]", correct.get(0).split("\t", -1)[9]);
    assertEquals("[6b/02/4/10]", defective.get(0).split("\t", -1)[9]);
    assertTrue(defective.get(0).contains("updated/6b/01/02/4/3"));
  }

  @Test
  void scheduleParserRejectsMalformedAndUnboundedInputs() {
    for (String invalid : List.of("", "imap-api-schedule-v1\t1\n", "imap-api-schedule-v2\t1\n",
        "imap-api-schedule-v1\t-1\n", "imap-api-schedule-v1\t18446744073709551616\n", "é", "x".repeat(16 * 1024 + 1),
        "imap-api-schedule-v1\t1\r\n")) {
      assertThrows(IllegalArgumentException.class, () -> IMapConcurrentRunner.parse(new StringReader(invalid)));
    }
    for (String row : List.of("1\tcompare\toverlap\t1", "0\tcompare\toverlap\t1\t2", "1\tbad\toverlap\t1\t2",
        "1\tcompare\tbad\t1\t2", "1\tcompare\toverlap\t0\t2", "1\tcompare\toverlap\t1\t1", "1\tcompare\toverlap\t256\t2",
        "1\tcancel_before\toverlap\t1\t2", "1\tcompare\toverlap\tx\t2")) {
      assertThrows(IllegalArgumentException.class, () -> IMapConcurrentRunner.parse(new StringReader(schedule(row))));
    }
    assertThrows(IllegalArgumentException.class, () -> IMapConcurrentRunner.parse(new StringReader(schedule(
        "1\tcompare\toverlap\t1\t2", "1\tcompare\toverlap\t1\t2"))));
    var rows = new ArrayList<String>();
    for (int index = 1; index <= 65; index++) rows.add(index + "\tcompare\toverlap\t1\t2");
    assertThrows(IllegalArgumentException.class,
        () -> IMapConcurrentRunner.parse(new StringReader(schedule(rows.toArray(String[]::new)))));
  }

  @Test
  void boundedScheduleAcceptsEofWithoutAnExtraEmptyRound() throws Exception {
    String source = schedule("1\tcompare\tleft_first\t1\t2");
    assertEquals(IMapConcurrentRunner.parse(new StringReader(source)),
        IMapConcurrentRunner.parse(new StringReader(source.substring(0, source.length() - 1))));
  }

  @Test
  void workerFaultsFailTheWholeRoundAndServiceErrorsKeepTheirTypedObservation() {
    var fault = new IllegalStateException("seed 117 worker checkpoint fault");
    var failure = assertThrows(IllegalStateException.class, () -> IMapConcurrentRunner.execute(
        new StringReader(schedule("1\tcompare\toverlap\t1\t2")), false,
        worker -> { if (worker == 0) throw fault; }));
    assertEquals("concurrent worker failed", failure.getMessage());
    assertEquals(fault, failure.getCause());
    var bounds = new BoundedIMapApi.Bounds(1024, 1024, 16, 16, 16, 4096, 16);
    var service = new UnifiedIMapService(bounds, 1);
    assertEquals("error:unsupported_operation|~|~|~|0|0|[]",
        IMapConcurrentRunner.result(service, new UnifiedIMapService.DomainCommand("unsupported")));
  }

  @Test
  void stdioProcessEntryPointAcceptsOnlyItsNamedDefect() throws Exception {
    var oldIn = System.in;
    var oldOut = System.out;
    try {
      String source = schedule("1\tcompare\tleft_first\t1\t2");
      var correct = new ByteArrayOutputStream();
      System.setIn(new ByteArrayInputStream(source.getBytes(StandardCharsets.US_ASCII)));
      System.setOut(new PrintStream(correct, true, StandardCharsets.US_ASCII));
      IMapConcurrentRunner.main(new String[0]);
      var defective = new ByteArrayOutputStream();
      System.setIn(new ByteArrayInputStream(source.getBytes(StandardCharsets.US_ASCII)));
      System.setOut(new PrintStream(defective, true, StandardCharsets.US_ASCII));
      IMapConcurrentRunner.main(new String[] {"--canary-unconditional-compare"});
      assertNotEquals(correct.toString(StandardCharsets.US_ASCII), defective.toString(StandardCharsets.US_ASCII));
      assertThrows(IllegalArgumentException.class, () -> IMapConcurrentRunner.main(new String[] {"--unknown"}));
    } finally { System.setIn(oldIn); System.setOut(oldOut); }
  }
}
