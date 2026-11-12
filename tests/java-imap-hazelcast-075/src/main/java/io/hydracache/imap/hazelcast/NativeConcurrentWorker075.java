package io.hydracache.imap.hazelcast;

import com.hazelcast.client.HazelcastClient;
import com.hazelcast.client.config.ClientConfig;
import com.hazelcast.core.EntryEvent;
import com.hazelcast.map.IMap;
import com.hazelcast.map.listener.EntryAddedListener;
import com.hazelcast.map.listener.EntryUpdatedListener;
import com.hazelcast.map.listener.EntryRemovedListener;
import io.hydracache.imap.semantic.BytesValue;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicLong;
import static io.hydracache.imap.hazelcast.NativeConcurrentScenario075.*;
import static io.hydracache.imap.hazelcast.NativeConcurrentHistory075.*;
import static io.hydracache.imap.hazelcast.NativeIMapObservation075.*;

/** Actual two-client cohort. No racing size/read admission and no follow-up get for conditionals. */
public final class NativeConcurrentWorker075 {
  private NativeConcurrentWorker075() {}
  public record Run(History history, List<NativeEvent> nativeEvents, UUID member, List<UUID> clients, int attempts) {
    public Run { nativeEvents = List.copyOf(nativeEvents); clients = List.copyOf(clients); }
  }
  public static Run qualified(HazelcastLoopback075 fixture, Program program, int maxAttempts) throws Exception {
    return qualified(maxAttempts,attempt->run(fixture,program,attempt));
  }
  @FunctionalInterface interface Attempt {Run execute(int attempt) throws Exception;}
  static Run qualified(int maxAttempts,Attempt attemptSource) throws Exception {
    if (maxAttempts < 1 || maxAttempts > 8) throw new IllegalArgumentException("native overlap attempt bound");
    for (int attempt = 1; attempt <= maxAttempts; attempt++) {
      var run = attemptSource.execute(attempt);
      if (run.history().hasOverlap()) {
        if (NativeConcurrentOracle075.check(run.history(), 100_000).verdict() != NativeConcurrentOracle075.Verdict.LINEARIZABLE) throw new IllegalStateException("native history failed qualification");
        return run;
      }
    }
    throw new IllegalStateException("no actual native overlap observed within attempt bound");
  }
  static Run run(HazelcastLoopback075 fixture, Program program, int attempt) throws Exception {
    var member = fixture.member().getCluster().getLocalMember();
    if (fixture.member().getCluster().getMembers().size() != 1) throw new IllegalStateException("native cohort topology changed");
    var config = new ClientConfig().setClusterName(fixture.member().getConfig().getClusterName());
    config.setProperty("hazelcast.logging.type", "none");
    config.setProperty(com.hazelcast.client.properties.ClientProperty.INVOCATION_TIMEOUT_SECONDS.getName(), "5");
    config.getNetworkConfig().addAddress("127.0.0.1:" + member.getAddress().getPort());
    config.getConnectionStrategyConfig().getConnectionRetryConfig().setClusterConnectTimeoutMillis(3000);
    var second = HazelcastClient.newHazelcastClient(config);
    var pool = Executors.newFixedThreadPool(2); var start = new CountDownLatch(1); var ready = new CountDownLatch(2);
    IMap<byte[], byte[]> map = fixture.client().getMap("imap075-concurrent-" + UUID.randomUUID());
    var events = new Events(); UUID listener = null;
    try {
      if (!map.isEmpty()) throw new IllegalStateException("native concurrent map must start empty");
      listener = map.addEntryListener(events, true);
      var raw = new ConcurrentLinkedQueue<Event>(); var stamps = new AtomicLong(); var futures = new ArrayList<Future<?>>();
      for (int worker = 0; worker < 2; worker++) {
        int identity = worker; IMap<byte[], byte[]> proxy = worker == 0 ? map : second.getMap(map.getName());
        futures.add(pool.submit(() -> {
          ready.countDown(); if (!start.await(5, TimeUnit.SECONDS)) throw new IllegalStateException("native start barrier expired");
          for (var command : program.commands()) if (command.worker() == identity) {
            raw.add(new Invoke(stamps.incrementAndGet(), command.id(), identity));
            var result = execute(proxy, command);
            raw.add(new Return(stamps.incrementAndGet(), command.id(), identity, result));
          }
          return null;
        }));
      }
      if (!ready.await(5, TimeUnit.SECONDS)) throw new IllegalStateException("native workers not ready"); start.countDown();
      for (var future : futures) future.get(10, TimeUnit.SECONDS);
      var entries = new ArrayList<Entry>();
      for (var key : program.keys()) {
        var view = map.getEntryView(key.copy());
        if (view != null) {
          if (view.getTtl() != 0 && view.getTtl() != Long.MAX_VALUE || view.getValue().length > 64) throw new IllegalStateException("native concurrent profile violation");
          entries.add(new Entry(key, BytesValue.copyOf(view.getValue())));
        }
      }
      if (map.size() != entries.size() || fixture.member().getCluster().getMembers().size() != 1
          || !fixture.member().getCluster().getLocalMember().getUuid().equals(member.getUuid())) throw new IllegalStateException("native concurrent ownership/topology changed");
      var history = History.observed(program, List.copyOf(raw), new Snapshot(entries, digestEntries(entries)));
      int expectedEvents = (int) history.calls().stream().filter(NativeConcurrentWorker075::emits).count();
      var observedEvents = events.await(expectedEvents);
      return new Run(history, observedEvents, member.getUuid(), List.of(fixture.client().getLocalEndpoint().getUuid(), second.getLocalEndpoint().getUuid()), attempt);
    } finally {
      start.countDown(); pool.shutdownNow(); second.shutdown();
      try { if (!pool.awaitTermination(3, TimeUnit.SECONDS)) throw new IllegalStateException("native worker cleanup incomplete"); }
      finally { if (listener != null) map.removeEntryListener(listener); }
    }
  }
  static RawResult execute(IMap<byte[], byte[]> map, Command command) {
    byte[] key = command.key().copy();
    return switch (command.operation()) {
      case GET -> RawResult.value(map.get(key));
      case CONTAINS_KEY -> RawResult.bool(map.containsKey(key));
      case PUT, GET_AND_PUT -> RawResult.value(map.put(key, command.value().orElseThrow().copy(), 0, TimeUnit.MILLISECONDS));
      case PUT_IF_ABSENT -> RawResult.value(map.putIfAbsent(key, command.value().orElseThrow().copy(), 0, TimeUnit.MILLISECONDS));
      case REPLACE_IF_PRESENT -> RawResult.value(map.replace(key, command.value().orElseThrow().copy()));
      case GET_AND_REMOVE -> RawResult.value(map.remove(key));
      case REPLACE_IF_VALUE -> RawResult.bool(map.replace(key, command.expected().orElseThrow().copy(), command.value().orElseThrow().copy()));
      case REMOVE_IF_VALUE -> RawResult.bool(map.remove(key, command.expected().orElseThrow().copy()));
    };
  }
  static boolean emits(Call call) {
    return switch (call.command().operation()) {
      case PUT, GET_AND_PUT -> true;
      case PUT_IF_ABSENT -> call.result().value().isEmpty();
      case REPLACE_IF_PRESENT, GET_AND_REMOVE -> call.result().value().isPresent();
      case REPLACE_IF_VALUE, REMOVE_IF_VALUE -> call.result().booleanValue().orElseThrow();
      case GET, CONTAINS_KEY -> false;
    };
  }
  private static final class Events implements EntryAddedListener<byte[], byte[]>, EntryUpdatedListener<byte[], byte[]>, EntryRemovedListener<byte[], byte[]> {
    final ArrayBlockingQueue<NativeEvent> queue = new ArrayBlockingQueue<>(32); volatile boolean overflow;
    void add(EntryEvent<byte[], byte[]> event, EventKind kind) {
      if (event.getKey().length > 64 || event.getValue() != null && event.getValue().length > 64 || event.getOldValue() != null && event.getOldValue().length > 64) { overflow = true; return; }
      if (!queue.offer(new NativeEvent(1, kind, optional(event.getKey()), optional(event.getOldValue()), optional(event.getValue()), 1))) overflow = true;
    }
    public void entryAdded(EntryEvent<byte[], byte[]> event) { add(event, EventKind.ADDED); }
    public void entryUpdated(EntryEvent<byte[], byte[]> event) { add(event, EventKind.UPDATED); }
    public void entryRemoved(EntryEvent<byte[], byte[]> event) { add(event, EventKind.REMOVED); }
    List<NativeEvent> await(int expected) throws InterruptedException {
      var result = new ArrayList<NativeEvent>(); long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
      while (result.size() < expected && System.nanoTime() < deadline) {
        var event = queue.poll(10, TimeUnit.MILLISECONDS); if (event != null) result.add(event);
      }
      queue.drainTo(result); if (overflow || result.size() != expected) throw new IllegalStateException("native concurrent callback count/gap"); return List.copyOf(result);
    }
  }
}
