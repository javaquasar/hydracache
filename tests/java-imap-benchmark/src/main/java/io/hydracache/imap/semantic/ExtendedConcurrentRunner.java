package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedConcurrentScenario.*;
import java.io.InputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Flow;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.BiConsumer;

/** Real two-worker local histories and an independent validation entry point. */
public final class ExtendedConcurrentRunner {
  private ExtendedConcurrentRunner() {}
  public static List<History> execute(ExtendedConcurrentScenario scenario, boolean defect) {
    return execute(scenario, defect, (worker, ordinal) -> {});
  }
  // Package-local failure checkpoint exercises worker propagation without inventing owner outcomes.
  static List<History> execute(ExtendedConcurrentScenario scenario, boolean defect, BiConsumer<Integer, Integer> checkpoint) {
    var result = new ArrayList<History>(); for (var round : scenario.rounds()) result.add(execute(round, defect, checkpoint)); return List.copyOf(result);
  }
  private static History execute(Round round, boolean defect, BiConsumer<Integer, Integer> checkpoint) {
    var owner = new Owner(round, defect); var executor = java.util.concurrent.Executors.newFixedThreadPool(2);
    var calls = Collections.synchronizedList(new ArrayList<Call>()); var stamps = new AtomicInteger();
    var ready = new CountDownLatch(2); var finished = new CountDownLatch[] {new CountDownLatch(1), new CountDownLatch(1)};
    try {
      var futures = new ArrayList<java.util.concurrent.Future<?>>();
      for (int worker = 0; worker < 2; worker++) { int actor = worker;
        futures.add(executor.submit(() -> {
          try {
            if (round.order().equals(actor == 0 ? "right_first" : "left_first")) await(finished[1 - actor]);
            var program = round.program(actor);
            for (int ordinal = 0; ordinal < program.size(); ordinal++) {
              int start = stamps.incrementAndGet();
              if (ordinal == 0 && round.order().equals("overlap")) { ready.countDown(); await(ready); }
              checkpoint.accept(actor, ordinal);
              String result; try { result = owner.action(program.get(ordinal)); } catch (IMapDomainException failure) { result = "error:" + failure.kind(); }
              calls.add(new Call(actor, ordinal, start, stamps.incrementAndGet(), result));
            }
          } finally { finished[actor].countDown(); }
        }));
      }
      for (var future : futures) future.get(3, TimeUnit.SECONDS);
      owner.close(); return new History(round, sorted(calls), owner.signature());
    } catch (Exception failure) { throw new IllegalStateException("bounded concurrent worker failed", failure); }
    finally { executor.shutdownNow(); owner.close(); }
  }
  private static void await(CountDownLatch latch) { try { if (!latch.await(3, TimeUnit.SECONDS)) throw new IllegalStateException("worker gate deadline"); } catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); throw new IllegalStateException("worker interrupted", interrupted); } }
  private static BytesValue b(String hex) { return BytesValue.fromHex(hex); }
  private static BoundedIMapApi.MutationIdentity id(String client, long sequence) { return new BoundedIMapApi.MutationIdentity(client, sequence); }
  private static String event(UnifiedIMapService.Event event) { return event.kind() + "/" + event.key().hex() + "/" + event.oldValue().map(BytesValue::hex).orElse("~") + "/" + event.value().map(BytesValue::hex).orElse("~") + "/" + event.watermark(); }
  static final class Subscriber implements Flow.Subscriber<UnifiedIMapService.Event> {
    final List<String> delivered = new java.util.concurrent.CopyOnWriteArrayList<>(); final AtomicReference<Throwable> failure = new AtomicReference<>(); Flow.Subscription subscription;
    public void onSubscribe(Flow.Subscription subscription) { this.subscription = subscription; }
    public void onNext(UnifiedIMapService.Event value) { delivered.add(event(value)); }
    public void onError(Throwable failure) { this.failure.set(failure); }
    public void onComplete() { failure.set(new IllegalStateException("live listener completed")); }
  }
  private static final class Owner implements AutoCloseable {
    final Round round; final boolean defect; final UnifiedIMapService service = new UnifiedIMapService(new BoundedIMapApi.Bounds(16, 16, 32, 8, 32, 4096, 16), 1);
    final UnifiedIMapService.ListenerHandle listener = service.registerListener(1, new UnifiedIMapService.ListenerOptions(true, 16, 8, Optional.empty(), java.util.Set.of()));
    final java.util.Map<BytesValue, UnifiedIMapService.VersionToken> captured = new java.util.TreeMap<>(); final AtomicBoolean reserved = new AtomicBoolean();
    IMapFlowPublisher<UnifiedIMapService.Event> publisher; Subscriber subscriber; UnifiedIMapService.SnapshotLease held; List<String> events = List.of(); boolean closed;
    Owner(Round round, boolean defect) {
      this.round = round; this.defect = defect;
      if (!round.family().equals("retire_pending")) {
        service.execute(put("61", "00", "setup", 1).ttl(round.family().equals("bulk_expiry") ? TtlDirective.expireAfter(1) : TtlDirective.eternal())); capture("61");
        if (!round.family().equals("checkpoint_lease")) { service.execute(put("62", "00", "setup", 2)); capture("62"); }
      }
      if (round.family().equals("listener_cut")) {
        publisher = service.events(listener); subscriber = new Subscriber(); publisher.subscribe(subscriber); subscriber.subscription.request(1);
        if (subscriber.delivered.size() != 1) throw new IllegalStateException("initial listener demand"); subscriber.delivered.clear();
      } else listener.drain();
      if (round.family().equals("checkpoint_lease")) { held = service.snapshotListener(1, 8, 4096, 0); listener.close(); }
    }
    void capture(String key) { captured.put(b(key), service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(b(key))).token().orElseThrow()); }
    UnifiedIMapService.DomainCommand put(String key, String value, String client, int sequence) { return new UnifiedIMapService.DomainCommand("put").key(b(key)).value(b(value)).identity(id(client, sequence)); }
    String action(String action) {
      return switch (action) {
        case "bulk" -> {
          var entries = new ArrayList<UnifiedIMapService.VersionInput>();
          for (String key : List.of("61", "62")) {
            var token = captured.get(b(key));
            if (defect && round.family().equals("bulk_aba")) token = service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(b(key))).token().orElse(token);
            entries.add(new UnifiedIMapService.VersionInput(b(key), token, Optional.of(b(key.equals("61") ? "01" : "02")), TtlDirective.preserve()));
          }
          var receipt = service.versionBulk(UnifiedIMapService.VersionAction.REPLACE, entries, id("bulk", 1), 0, Long.MAX_VALUE, false);
          yield receipt.kind() + ":" + String.join(",", receipt.items().stream().map(UnifiedIMapService.Item::kind).toList());
        }
        case "remove" -> service.execute(new UnifiedIMapService.DomainCommand("get_and_remove").key(b("61")).identity(id("aba", 1))).kind();
        case "put" -> service.execute(round.family().equals("listener_cut") ? put("63", "03", "event", 1) : put("61", "00", "aba", 2)).kind();
        case "expire" -> { var result = service.expireStep(1, 1); yield "expiry:" + result.work() + ":" + result.expired() + ":" + (result.remaining() ? 1 : 0); }
        case "reserve" -> { service.beginOperation(put("61", "01", "race", 1)); reserved.set(true); yield "pending"; }
        case "finish" -> reserved.get() ? service.executeReserved(put("61", "01", "race", 1)).kind() : "skipped";
        case "query" -> { var result = service.queryOperationOutcome(id("race", 1)); yield result.kind() + result.result().map(value -> ":" + value.kind()).orElse(""); }
        case "retire" -> "retired:" + service.retireOutcomes("race", 1, 1).removed();
        case "request" -> { subscriber.subscription.request(1); if (subscriber.failure.get() != null) throw new IllegalStateException("listener failed", subscriber.failure.get()); yield "requested"; }
        case "snapshot" -> { try (var lease = service.snapshotListener(1, 8, 4096, 0)) { yield "snapshot:" + lease.watermark() + ":" + String.join(",", lease.entries().stream().map(entry -> entry.key().hex()).toList()); } }
        case "close" -> { held.close(); yield "closed"; }
        default -> { try (var checkpoint = service.checkpoint()) { yield "checkpoint"; } }
      };
    }
    String signature() {
      long time = round.family().equals("bulk_expiry") ? 1 : 0;
      var rows = service.diagnosticSnapshot(time).stream().map(entry -> {
        var current = service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(entry.key()).now(time));
        return entry.key().hex() + "/" + entry.value().hex() + "/" + (entry.remainingTtl().isPresent() ? "x" + entry.remainingTtl().getAsLong() : "e") + "/" + (current.token().equals(Optional.ofNullable(captured.get(entry.key()))) ? 1 : 0);
      }).toList();
      var stats = service.diagnosticStats(time);
      return String.join(",", rows) + "|" + String.join(",", events) + "|" + (subscriber == null ? "" : String.join(",", subscriber.delivered)) + "|" + stats.retainedOutcomes() + "/" + stats.inflight() + "/" + stats.listeners() + "/" + stats.activeStreams() + "/" + (service.resourceUsage().resync() == 0 ? 0 : 1);
    }
    public void close() {
      if (closed) return; closed = true;
      if (publisher != null) publisher.close(); else events = listener.drain().stream().map(ExtendedConcurrentRunner::event).toList();
      listener.close(); if (held != null) held.close();
    }
  }
  public static void main(String[] args) throws Exception { run(args, System.in, System.out); }
  static void run(String[] args, InputStream input, PrintStream output) throws Exception {
    String text;
    if (args.length == 3 && args[0].equals("--generate")) text = generate(Long.parseLong(args[1]), Integer.parseInt(args[2])).encode();
    else {
      boolean check = args.length >= 2 && args[0].equals("--check-history");
      boolean defect = args.length == 3 && args[2].equals("--canary-unconditional-version");
      int budget = check && args.length == 4 && args[2].equals("--search-budget") ? Integer.parseInt(args[3]) : 4096;
      if (args.length < 2 || !(check || args[0].equals("--schedule")) || !(args.length == 2 || !check && defect || check && args.length == 4 && args[2].equals("--search-budget"))) throw new IllegalArgumentException("extended concurrent arguments");
      String source;
      if (args[1].equals("-")) source = read(input, check ? MAX_HISTORY_BYTES : MAX_SCHEDULE_BYTES);
      else try (var file = Files.newInputStream(Path.of(args[1]))) { source = read(file, check ? MAX_HISTORY_BYTES : MAX_SCHEDULE_BYTES); }
      if (check) {
        var histories = parseHistories(source); var result = new StringBuilder("imap-extended-verdict-v1\t" + histories.seed() + "\t" + histories.values().size() + "\n");
        for (var history : histories.values()) { var report = new ExtendedConcurrentOracle().validate(history, budget); result.append(history.round().id()).append('\t').append(report.verdict().name().toLowerCase(java.util.Locale.ROOT)).append('\t').append(report.explored()).append('\n'); }
        text = result.toString();
      } else { var scenario = parse(source); text = histories(scenario.seed(), execute(scenario, defect)); }
    }
    byte[] bytes = text.getBytes(StandardCharsets.UTF_8); output.write(bytes, 0, bytes.length); if (output.checkError()) throw new IllegalStateException("history output failed");
  }
  private static String read(InputStream input, int maximum) throws Exception { byte[] bytes = input.readNBytes(maximum + 1); if (bytes.length > maximum) throw new IllegalArgumentException("history input bound"); for (byte b : bytes) if (b < 0) throw new IllegalArgumentException("history input ASCII"); return new String(bytes, StandardCharsets.US_ASCII); }
}
