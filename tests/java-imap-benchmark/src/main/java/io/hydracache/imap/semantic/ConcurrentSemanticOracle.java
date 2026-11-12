package io.hydracache.imap.semantic;

import java.util.List;
import java.util.Optional;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;

/** Barrier-started atomicity probe whose verdict is independent of winner ordering. */
public final class ConcurrentSemanticOracle {
  public record RaceReport(List<OperationOutcome> outcomes, List<MapEvent> events,
                           StateSnapshot snapshot) {
    public RaceReport {
      outcomes = List.copyOf(outcomes);
      events = List.copyOf(events);
    }

    public void requireExactlyOnePutIfAbsentWinner() {
      long winners = outcomes.stream().filter(value -> value.kind() == OutcomeKind.INSERTED).count();
      long losers = outcomes.stream().filter(value -> value.kind() == OutcomeKind.PRESENT).count();
      long additions = events.stream().filter(value -> value.kind() == MapEvent.Kind.ADDED).count();
      if (winners != 1 || losers != 1 || additions != 1 || snapshot.liveCardinality() != 1) {
        throw new SemanticMismatchException(
            "put-if-absent race is not atomic: outcomes=" + outcomes + ", events=" + events);
      }
    }
  }

  public RaceReport putIfAbsentRace(
      MapSemanticAdapter adapter, BytesValue key, BytesValue left, BytesValue right)
      throws Exception {
    var ready = new CountDownLatch(2);
    var start = new CountDownLatch(1);
    ExecutorService executor = Executors.newFixedThreadPool(2);
    try {
      Future<OperationOutcome> first = executor.submit(() -> execute(adapter, ready, start,
          step("race-left", key, left)));
      Future<OperationOutcome> second = executor.submit(() -> execute(adapter, ready, start,
          step("race-right", key, right)));
      ready.await();
      start.countDown();
      return new RaceReport(List.of(first.get(), second.get()), adapter.drainEvents(),
          adapter.snapshot());
    } finally {
      executor.shutdownNow();
    }
  }

  private static OperationOutcome execute(MapSemanticAdapter adapter, CountDownLatch ready,
      CountDownLatch start, ScenarioStep step) throws InterruptedException {
    ready.countDown();
    start.await();
    return adapter.execute(step);
  }

  private static ScenarioStep step(String id, BytesValue key, BytesValue value) {
    return new ScenarioStep(id, Operation.PUT_IF_ABSENT, List.of(),
        List.of(new ScenarioEntry(key, value)), Optional.empty(), TtlDirective.eternal(), 0);
  }
}
