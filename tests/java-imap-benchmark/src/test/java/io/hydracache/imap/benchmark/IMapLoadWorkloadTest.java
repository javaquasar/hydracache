package io.hydracache.imap.benchmark;

import static org.junit.jupiter.api.Assertions.*;

import io.hydracache.imap.semantic.BytesValue;
import io.hydracache.imap.semantic.Operation;
import io.hydracache.imap.semantic.ScenarioEntry;
import io.hydracache.imap.semantic.ScenarioManifest;
import io.hydracache.imap.semantic.ScenarioStep;
import io.hydracache.imap.semantic.TtlDirective;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Optional;
import java.util.stream.IntStream;
import org.junit.jupiter.api.Test;

final class IMapLoadWorkloadTest {
  private static final BytesValue KEY = BytesValue.fromHex("00ff");
  private static final ScenarioStep GET = ScenarioStep.get("get", KEY);

  @Test void immutablePreloadAndWeightedRequestPool() {
    var write = write("put", KEY, BytesValue.fromHex(""), Operation.PUT, Optional.empty());
    var preload = new ArrayList<>(List.of(write));
    var requests = new ArrayList<>(List.of(GET, write, GET));
    var workload = new IMapLoadWorkload(trace(write, GET), preload, requests);
    preload.clear();
    requests.clear();
    assertEquals(List.of(write), workload.preload());
    assertEquals(List.of(GET, write, GET), workload.requests());
    assertThrows(UnsupportedOperationException.class, () -> workload.requests().clear());
  }

  @Test void requiresBoundedPreflightAndExactTimedRequests() {
    var tooMany = IntStream.rangeClosed(0, 1024)
        .mapToObj(i -> ScenarioStep.get("read-" + i, KEY)).toList();
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(new ScenarioManifest(1, tooMany), List.of(), List.of(GET)));
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(trace(GET), List.of(), List.of()));
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(trace(GET), List.of(), Collections.nCopies(1025, GET)));
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(trace(GET), List.of(GET, GET), List.of(GET)));
    var different = ScenarioStep.get("get", BytesValue.fromHex("fe"));
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(trace(GET), List.of(different), List.of(GET)));
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(trace(GET), List.of(), List.of(different)));
  }

  @Test void boundsRequestPayloadAndPreflightFootprint() {
    reject(ScenarioStep.get("r".repeat(129), KEY));
    reject(ScenarioStep.get("large-key", BytesValue.copyOf(new byte[1025])));
    reject(new ScenarioStep("keys", Operation.GET_ALL, Collections.nCopies(257, KEY),
        List.of(), Optional.empty(), TtlDirective.preserve(), 0));
    var item = new ScenarioEntry(KEY, BytesValue.fromHex("ff"));
    reject(new ScenarioStep("entries", Operation.PUT_ALL, List.of(), Collections.nCopies(257, item),
        Optional.empty(), TtlDirective.eternal(), 0));
    var tooLarge = BytesValue.copyOf(new byte[1024 * 1024 + 1]);
    reject(write("large-value", KEY, tooLarge, Operation.PUT, Optional.empty()));
    reject(write("large-expected", KEY, item.value(), Operation.REPLACE, Optional.of(tooLarge)));
    var large = BytesValue.copyOf(new byte[1024 * 1024]);
    var trace = new ArrayList<>(List.of(GET));
    for (int i = 0; i < 9; i++) {
      trace.add(write("put-" + i, KEY, large, Operation.PUT, Optional.empty()));
    }
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(new ScenarioManifest(2, trace), List.of(), List.of(GET)));
    var compare = write("compare", KEY, item.value(), Operation.REPLACE, Optional.of(item.value()));
    assertEquals(List.of(compare), new IMapLoadWorkload(trace(compare), List.of(), List.of(compare)).requests());
  }

  @Test void controlsAndDuplicateKeysBelongOnlyInSemanticPreflight() {
    var advance = new ScenarioStep("advance", Operation.ADVANCE, List.of(), List.of(),
        Optional.empty(), TtlDirective.preserve(), 1);
    var gap = new ScenarioStep("gap", Operation.LISTENER_GAP, List.of(KEY), List.of(),
        Optional.empty(), TtlDirective.preserve(), 0);
    var duplicates = new ScenarioStep("duplicates", Operation.GET_ALL, List.of(KEY, KEY),
        List.of(), Optional.empty(), TtlDirective.preserve(), 0);
    for (var step : List.of(advance, gap, duplicates)) {
      assertThrows(IllegalArgumentException.class,
          () -> new IMapLoadWorkload(trace(GET, step), List.of(), List.of(step)));
    }
    assertEquals(List.of(GET), new IMapLoadWorkload(
        trace(GET, advance, gap, duplicates), List.of(), List.of(GET)).requests());
    var item = new ScenarioEntry(KEY, BytesValue.fromHex("01"));
    var repeated = new ScenarioStep("duplicate-entries", Operation.PUT_ALL, List.of(),
        List.of(item, item), Optional.empty(), TtlDirective.eternal(), 0);
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(trace(repeated, GET), List.of(repeated), List.of(GET)));
  }

  private static void reject(ScenarioStep step) {
    assertThrows(IllegalArgumentException.class,
        () -> new IMapLoadWorkload(trace(step), List.of(), List.of(step)));
  }

  static ScenarioManifest trace(ScenarioStep... steps) {
    return new ScenarioManifest(117, List.of(steps));
  }

  static ScenarioStep write(String id, BytesValue key, BytesValue value, Operation operation,
                            Optional<BytesValue> expected) {
    return new ScenarioStep(id, operation, List.of(), List.of(new ScenarioEntry(key, value)),
        expected, TtlDirective.eternal(), 0);
  }
}
