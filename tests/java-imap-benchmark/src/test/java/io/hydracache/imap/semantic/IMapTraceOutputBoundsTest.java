package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertThrows;
import java.io.StringReader;
import org.junit.jupiter.api.Test;

final class IMapTraceOutputBoundsTest {
  private static String source(int entries, int reads) {
    var source = new StringBuilder("imap-api-trace-v1\t117\n");
    for (int index = 0; index < entries; index++) {
      String key = String.format("%02x", index) + "00".repeat(1023);
      source.append(String.join("\t", "set", key, "ff".repeat(1024), "~", "eternal",
          Integer.toString(index + 1), "0", "1000", "", "")).append('\n');
    }
    for (int index = 0; index < reads; index++) {
      source.append("get\t\t~\t~\teternal\t0\t0\t1000\t\t\n");
    }
    return source.toString();
  }

  @Test
  void observationAndAggregateOutputBudgetsStopRetention() {
    assertThrows(IllegalArgumentException.class,
        () -> IMapTraceRunner.execute(new StringReader(source(17, 0)), false));
    assertThrows(IllegalArgumentException.class,
        () -> IMapTraceRunner.execute(new StringReader(source(15, 100)), false));
  }
}
