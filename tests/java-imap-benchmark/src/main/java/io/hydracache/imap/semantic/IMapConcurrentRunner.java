package io.hydracache.imap.semantic;

import java.io.IOException;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Executor;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;

/** Independent process peer for bounded, explicitly gated concurrent service histories. */
public final class IMapConcurrentRunner {
  private static final int MAX_BYTES = 16 * 1024;
  private static final int MAX_ROUNDS = 64;
  private static final int MAX_OUTPUT = 256 * 1024;
  private static final BytesValue KEY = BytesValue.fromHex("6b");
  private IMapConcurrentRunner() {}

  public static void main(String[] args) throws Exception {
    boolean canary = args.length == 1 && args[0].equals("--canary-unconditional-compare");
    if (args.length > 0 && !canary) throw new IllegalArgumentException("unknown concurrent-runner option");
    for (String line : execute(new InputStreamReader(System.in, StandardCharsets.US_ASCII), canary)) {
      System.out.println(line);
    }
  }

  record Round(long id, String kind, String order, int left, int right) {}

  static List<Round> parse(Reader input) throws IOException {
    var source = new StringBuilder();
    int value;
    while ((value = input.read()) != -1) {
      if (source.length() == MAX_BYTES || value > 127 || value == '\r') {
        throw new IllegalArgumentException("invalid or oversized concurrent schedule");
      }
      source.append((char) value);
    }
    String[] lines = source.toString().split("\n", -1);
    if (lines.length < 2 || !lines[0].startsWith("imap-api-schedule-v1\t")) {
      throw new IllegalArgumentException("unsupported concurrent schedule header");
    }
    number(lines[0].substring("imap-api-schedule-v1\t".length()), true);
    var result = new ArrayList<Round>();
    var ids = new HashSet<Long>();
    for (int index = 1; index < lines.length; index++) {
      if (index == lines.length - 1 && lines[index].isEmpty()) break;
      String[] fields = lines[index].split("\t", -1);
      if (fields.length != 5 || result.size() == MAX_ROUNDS) {
        throw new IllegalArgumentException("concurrent schedule field or round bound");
      }
      long id = number(fields[0], false), left = number(fields[3], false), right = number(fields[4], false);
      if (!ids.add(id) || !List.of("compare", "duplicate", "expiry", "cancel_before", "cancel_after").contains(fields[1])
          || !List.of("overlap", "left_first", "right_first").contains(fields[2])
          || left > 255 || right > 255 || left == right
          || (fields[1].startsWith("cancel_") && !fields[2].equals("left_first"))) {
        throw new IllegalArgumentException("invalid concurrent round");
      }
      result.add(new Round(id, fields[1], fields[2], (int) left, (int) right));
    }
    if (result.isEmpty()) throw new IllegalArgumentException("concurrent schedule has no rounds");
    return List.copyOf(result);
  }

  public static List<String> execute(Reader input, boolean canary) throws Exception {
    return execute(input, canary, worker -> { });
  }

  static List<String> execute(Reader input, boolean canary, java.util.function.IntConsumer checkpoint) throws Exception {
    var output = new ArrayList<String>();
    int bytes = 0;
    for (Round round : parse(input)) {
      var service = setup();
      History history = round.kind().startsWith("cancel_") ? asynchronous(round, service) : race(round, service, canary, checkpoint);
      String line = history.encode();
      bytes += line.length() + 1;
      if (line.length() > 4096 || bytes > MAX_OUTPUT) throw new IllegalArgumentException("concurrent history output bound");
      output.add(line);
    }
    return List.copyOf(output);
  }

  private static UnifiedIMapService setup() {
    var bounds = new BoundedIMapApi.Bounds(1024 * 1024, 4 * 1024 * 1024, 4096,
        256, 4096, 8 * 1024 * 1024, 4096);
    var service = new UnifiedIMapService(bounds, 1);
    service.execute(new UnifiedIMapService.DomainCommand("entry_listener").listenerId(1).includeValue(true));
    service.execute(new UnifiedIMapService.DomainCommand("put").key(KEY).value(bytes(0))
        .ttl(TtlDirective.expireAfter(10)).identity(new BoundedIMapApi.MutationIdentity("setup", 1)));
    service.drainListener(1);
    return service;
  }

  private static UnifiedIMapService.DomainCommand command(Round round, int index, boolean canary) {
    String operation = switch (round.kind()) { case "compare" -> "conditional_replace"; case "duplicate" -> "get_and_put"; default -> "get"; };
    if (canary && round.kind().equals("compare") && index == 1) operation = "get_and_put";
    return new UnifiedIMapService.DomainCommand(operation).key(KEY).expected(bytes(0))
        .value(bytes(index == 0 || round.kind().equals("duplicate") ? round.left() : round.right()))
        .ttl(TtlDirective.preserve()).now(round.kind().equals("expiry") ? 10 : 0)
        .identity(new BoundedIMapApi.MutationIdentity("race", round.kind().equals("duplicate") ? 1 : index + 1));
  }

  private static final class History {
    final Round round;
    final long[] begin = {1, 3}, end = {2, 4};
    final String[] results = new String[2];
    String snapshot, events, replay, query, asynchronous = "none", resources;
    History(Round round) { this.round = round; }
    String encode() {
      return String.join("\t", Long.toString(round.id()), round.kind(), round.order(), Long.toString(begin[0]),
          Long.toString(end[0]), results[0], Long.toString(begin[1]), Long.toString(end[1]), results[1],
          snapshot, events, replay, query, asynchronous, resources);
    }
  }

  private static History race(Round round, UnifiedIMapService service, boolean canary,
      java.util.function.IntConsumer checkpoint) throws Exception {
    var history = new History(round);
    var tickets = new AtomicLong(1);
    var ready = new CountDownLatch(2);
    CountDownLatch[] starts = {new CountDownLatch(1), new CountDownLatch(1)};
    CountDownLatch[] applies = {new CountDownLatch(1), new CountDownLatch(1)};
    CountDownLatch[] started = {new CountDownLatch(1), new CountDownLatch(1)};
    CountDownLatch[] done = {new CountDownLatch(1), new CountDownLatch(1)};
    var failure = new AtomicReference<Throwable>();
    var workers = new ArrayList<Thread>();
    for (int index = 0; index < 2; index++) {
      final int worker = index;
      Thread thread = new Thread(() -> {
        try {
          ready.countDown(); await(starts[worker]);
          history.begin[worker] = tickets.getAndIncrement(); started[worker].countDown();
          await(applies[worker]);
          checkpoint.accept(worker);
          history.results[worker] = result(service, command(round, worker, canary));
          history.end[worker] = tickets.getAndIncrement();
        } catch (Throwable error) { failure.compareAndSet(null, error); }
        finally { done[worker].countDown(); }
      }, "imap-qualification-" + index);
      thread.setDaemon(true); workers.add(thread); thread.start();
    }
    await(ready);
    if (round.order().equals("overlap")) {
      for (var gate : starts) gate.countDown();
      for (var gate : started) await(gate);
      for (var gate : applies) gate.countDown();
      for (var gate : done) await(gate);
    } else {
      int first = round.order().equals("right_first") ? 1 : 0;
      for (int index : new int[] {first, 1 - first}) {
        starts[index].countDown(); await(started[index]); applies[index].countDown(); await(done[index]);
      }
    }
    for (Thread worker : workers) join(worker);
    if (failure.get() != null) throw new IllegalStateException("concurrent worker failed", failure.get());
    int winner = history.results[0].startsWith("replaced|") || !round.kind().equals("compare") ? 0 : 1;
    var replay = command(round, winner, canary);
    history.replay = result(service, replay);
    history.query = query(service, replay.identity.orElseThrow(), replay.now);
    finish(service, history, replay.now);
    return history;
  }

  /** The real facade queues admission and application independently; tests release each phase. */
  private static final class ControlledExecutor implements Executor {
    private final ArrayDeque<Runnable> work = new ArrayDeque<>();
    @Override public synchronized void execute(Runnable command) {
      if (work.size() == 2) throw new RejectedExecutionException("controlled executor capacity");
      work.add(command);
    }
    void runOne() throws Exception {
      Runnable task;
      synchronized (this) { task = work.poll(); }
      if (task == null) throw new IllegalStateException("missing async checkpoint");
      var failure = new AtomicReference<Throwable>();
      Thread worker = new Thread(() -> { try { task.run(); } catch (Throwable error) { failure.set(error); } }, "imap-async-checkpoint");
      worker.setDaemon(true); worker.start(); join(worker);
      if (failure.get() != null) throw new IllegalStateException("async worker failed", failure.get());
    }
  }

  private static History asynchronous(Round round, UnifiedIMapService service) throws Exception {
    var history = new History(round);
    var executor = new ControlledExecutor();
    var facade = new UnifiedIMapAsyncFacade(service, executor, () -> 0, 1);
    var identity = new BoundedIMapApi.MutationIdentity("race", 1);
    var command = new UnifiedIMapService.DomainCommand("set").key(KEY).value(bytes(round.left()))
        .ttl(TtlDirective.preserve()).identity(identity).deadline(100);
    var invocation = facade.invoke(command);
    if (round.kind().equals("cancel_before")) {
      if (!invocation.cancel()) throw new IllegalStateException("pre-admission cancellation rejected");
      executor.runOne();
      history.results[0] = canonical(invocation.result().toCompletableFuture().get(3, TimeUnit.SECONDS));
      history.begin[1] = 0; history.end[1] = 0; history.results[1] = "-";
      history.replay = "not_run";
      history.asynchronous = "cancelled_before/not_applied/unknown";
    } else {
      executor.runOne();
      if (!query(service, identity, 0).equals("pending|~|~|~|0|0|[]")) throw new IllegalStateException("admitted operation not pending");
      if (!invocation.cancel()) throw new IllegalStateException("post-admission cancellation rejected");
      history.results[0] = canonical(invocation.result().toCompletableFuture().get(3, TimeUnit.SECONDS));
      executor.runOne();
      history.results[1] = canonical(service.recoverOperation(command).orElseThrow());
      var retry = facade.invoke(command);
      executor.runOne();
      history.replay = canonical(retry.result().toCompletableFuture().get(3, TimeUnit.SECONDS));
      history.asynchronous = "cancelled_after/outcome_unknown/pending";
    }
    if (facade.pending() != 0) throw new IllegalStateException("async capacity leaked");
    history.query = query(service, identity, 0);
    finish(service, history, 0);
    return history;
  }

  private static void finish(UnifiedIMapService service, History history, long now) {
    history.snapshot = "[" + service.diagnosticSnapshot(now).stream().map(entry -> entry.key().hex() + "/" + entry.value().hex()
        + "/" + entry.version() + "/" + number(entry.remainingTtl())).collect(Collectors.joining(",")) + "]";
    history.events = "[" + service.drainListener(1).stream().map(event -> event.kind() + "/" + event.key().hex() + "/"
        + optional(event.oldValue()) + "/" + optional(event.value()) + "/" + event.version() + "/" + event.watermark())
        .collect(Collectors.joining(",")) + "]";
    var stats = service.diagnosticStats(now);
    history.resources = "[" + stats.liveEntries() + "," + stats.valueBytes() + "," + stats.retainedOutcomes() + ","
        + stats.inflight() + "," + stats.listeners() + "," + stats.queuedEvents() + "]";
  }

  private static String query(UnifiedIMapService service, BoundedIMapApi.MutationIdentity identity, long now) {
    return result(service, new UnifiedIMapService.DomainCommand("query_operation_outcome").identity(identity).now(now));
  }
  static String result(UnifiedIMapService service, UnifiedIMapService.DomainCommand command) {
    try { return canonical(service.execute(command)); }
    catch (IMapDomainException error) { return "error:" + error.kind() + "|~|~|~|0|0|[]"; }
  }
  private static String canonical(UnifiedIMapService.Result result) {
    if (!result.items().isEmpty()) throw new IllegalArgumentException("unexpected concurrent bulk result");
    return result.kind() + "|" + optional(result.value()) + "|" + number(result.version()) + "|" + number(result.remainingTtl())
        + "|" + result.number() + "|" + (result.flag() ? 1 : 0) + "|[]";
  }
  private static BytesValue bytes(int value) { return BytesValue.copyOf(new byte[] {(byte) value}); }
  private static String optional(Optional<BytesValue> value) { return value.map(BytesValue::hex).orElse("~"); }
  private static String number(OptionalLong value) { return value.isPresent() ? Long.toString(value.getAsLong()) : "~"; }
  private static long number(String value, boolean zero) {
    if (!value.matches("[0-9]+")) throw new IllegalArgumentException("invalid concurrent numeric field");
    long result = Long.parseLong(value);
    if (!zero && result == 0) throw new IllegalArgumentException("concurrent numeric bound");
    return result;
  }
  private static void await(CountDownLatch gate) throws InterruptedException {
    if (!gate.await(3, TimeUnit.SECONDS)) throw new IllegalStateException("concurrent checkpoint timeout");
  }
  private static void join(Thread worker) throws InterruptedException {
    worker.join(3000);
    if (worker.isAlive()) { worker.interrupt(); throw new IllegalStateException("concurrent worker timeout"); }
  }
}
