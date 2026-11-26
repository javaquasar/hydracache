package io.hydracache.hazelcast;

import io.hydracache.client.hc2.BatchItemReceipt;
import io.hydracache.client.hc2.Capability;
import io.hydracache.client.hc2.ErrorCode;
import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.HydraCacheClientConfig;
import io.hydracache.client.hc2.HydraCacheException;
import io.hydracache.client.hc2.RequestOptions;
import io.hydracache.client.hc2.SubscriptionGap;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.EnumSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.function.Predicate;
import java.util.function.Supplier;

/** Started by the Rust three-member process test with the two packaged JARs. */
public final class W10gPackagedFacadeProbe {
  private static final Duration DEADLINE = Duration.ofSeconds(10);
  private static final RequestOptions READ = RequestOptions.withTimeout(Duration.ofSeconds(2));

  public static void main(String[] args) throws Exception {
    requireJar(HydraMap.class);
    requireJar(HydraCacheClient.class);
    Path root = Path.of(args[0]);
    String hot = args[4], control = args[5], third = args[6];
    List<String> keys = List.of(third, hot, control);
    try (var first = connect(root, args[1]); var second = connect(root, args[2]);
         var thirdEndpoint = connect(root, args[3])) {
      var map = map(thirdEndpoint);
      var observer = new Observer();
      try (var registration = map.addEntryListener(observer)) {
        var seed = new LinkedHashMap<String, String>();
        for (String key : keys) seed.put(key, key);
        var sender = map(second);
        var initial = sender.putAll(seed, new RequestOptions(
            DEADLINE, "java-w10g-seed".getBytes(StandardCharsets.UTF_8)));
        require(initial.stream().allMatch(item -> item.outcome().receipt().orElseThrow().state()
            == BatchItemReceipt.State.CONFIRMED), "initial puts were not confirmed");
        var initialValues = map(first).getAll(keys, READ);
        require(initialValues.stream().allMatch(item -> item.value().orElseThrow().equals(item.key())),
            "first endpoint did not read all values");
        for (String key : keys) {
          require(observer.awaitEvent(key, key) > 0, "initial event missing");
        }
        Files.createFile(root.resolve("java-ready"));
        var partial = awaitBatch(map, keys, result -> result.get(0).value().isPresent()
            && result.get(1).outcome().error().isPresent() && result.get(2).value().isPresent());
        require(partial.get(1).outcome().receipt().orElseThrow().state()
            != BatchItemReceipt.State.CONFIRMED, "failed owner was reported confirmed");
        awaitBatch(map, keys, result -> result.stream().allMatch(item -> item.value().isPresent())
            && result.get(1).outcome().topologyEpoch() > initial.get(1).outcome().topologyEpoch());
        var gap = observer.gaps.poll(DEADLINE.toSeconds(), TimeUnit.SECONDS);
        require(gap != null && gap.reason() == SubscriptionGap.Reason.TOPOLOGY_CHANGED,
            "topology gap was not delivered through the facade");
        awaitMutation(() -> sender.putAll(Map.of(control, "after-gap"), READ));
        long watermark = observer.awaitEvent(control, "after-gap");
        Files.createFile(root.resolve("java-promoted"));
        long stop = System.nanoTime() + DEADLINE.toNanos();
        while (!Files.exists(root.resolve("java-rejoined"))) {
          require(System.nanoTime() < stop, "old owner did not rejoin");
          Thread.sleep(20);
        }
        try (var rejoined = connect(root, args[1])) {
          var resumed = map(rejoined);
          var resumedObserver = new Observer();
          try (var resumedRegistration = resumed.addEntryListener(watermark, resumedObserver)) {
            var resume = resumedObserver.gaps.poll(DEADLINE.toSeconds(), TimeUnit.SECONDS);
            require(resume != null && resume.reason() == SubscriptionGap.Reason.RESUME_UNBOUND
                && resume.partitions().isEmpty(), "reconnect did not signal stream-wide repair");
            awaitMutation(() -> sender.removeAll(List.of(control), READ));
            require(resumedObserver.awaitEvent(control, null) > watermark,
                "reconnected listener did not receive the delete");
          }
        }
      }
    }
    System.out.println("W10g packaged Java facade partial_batch=true topology_gap=true resume_gap=true");
  }

  private static void awaitMutation(Supplier<List<MapBatchItem<String, String>>> operation)
      throws InterruptedException {
    long stop = System.nanoTime() + DEADLINE.toNanos();
    do {
      var result = operation.get().get(0).outcome();
      if (result.result().isPresent()) {
        require(result.result().orElseThrow().mutation().orElseThrow().applied(), "mutation did not apply");
        return;
      }
      var error = result.error().orElseThrow();
      if (result.receipt().orElseThrow().state() != BatchItemReceipt.State.NOT_APPLIED) throw error;
      requireRetryable(error);
      Thread.sleep(40);
    } while (System.nanoTime() < stop);
    throw new AssertionError("certified mutation was unavailable");
  }

  private static List<MapBatchItem<String, String>> awaitBatch(
      HydraMap<String, String> map, List<String> keys,
      Predicate<List<MapBatchItem<String, String>>> ready) throws InterruptedException {
    long stop = System.nanoTime() + DEADLINE.toNanos();
    do {
      try {
        var rows = map.getAll(keys, READ);
        require(rows.stream().map(MapBatchItem::key).toList().equals(keys), "batch order changed");
        if (ready.test(rows)) return rows;
      } catch (HydraCacheException error) {
        requireRetryable(error);
      }
      Thread.sleep(20);
    } while (System.nanoTime() < stop);
    throw new AssertionError("batch did not reach the expected state");
  }

  private static void requireRetryable(HydraCacheException error) {
    if (error.code() != ErrorCode.UNAVAILABLE && error.code() != ErrorCode.DEADLINE_EXCEEDED) {
      throw error;
    }
  }

  private static HydraCacheClient connect(Path root, String endpoint) {
    return HydraCacheClient.connect(HydraCacheClientConfig.builder()
        .endpoint(URI.create("https://" + endpoint)).serverName("localhost")
        .trustCertificate(root.resolve("clients.pem"))
        .clientCertificate(root.resolve("java-client.pem")).clientPrivateKey(root.resolve("java-client.key"))
        .tenant("w10i").connectTimeout(DEADLINE).defaultRequestTimeout(DEADLINE)
        .requestedCapabilities(EnumSet.of(Capability.DATA, Capability.BATCH, Capability.SUBSCRIPTIONS))
        .build());
  }

  private static HydraMap<String, String> map(HydraCacheClient client) {
    return new HydraCacheInstance(client, DEADLINE).getMap("w10g-map", HydraCodec.utf8(), HydraCodec.utf8());
  }

  private static void requireJar(Class<?> api) {
    require(api.getProtectionDomain().getCodeSource().getLocation().getPath().endsWith(".jar"),
        api.getName() + " did not come from a packaged JAR");
  }

  private static void require(boolean condition, String detail) {
    if (!condition) throw new AssertionError(detail);
  }

  private static final class Observer implements EntryListener<String, String> {
    private final ArrayBlockingQueue<EntryEvent<String, String>> events = new ArrayBlockingQueue<>(64);
    private final ArrayBlockingQueue<SubscriptionGap> gaps = new ArrayBlockingQueue<>(8);
    private final java.util.ArrayList<EntryEvent<String, String>> held = new java.util.ArrayList<>();

    @Override public void onEvent(EntryEvent<String, String> event) { events.add(event); }
    @Override public void onGap(long afterWatermark) {
      throw new AssertionError("structured facade Gap was not provided");
    }
    @Override public void onGap(long afterWatermark, SubscriptionGap gap) { gaps.add(gap); }

    long awaitEvent(String key, String value) throws InterruptedException {
      long stop = System.nanoTime() + DEADLINE.toNanos();
      do {
        for (int index = 0; index < held.size(); index++) {
          var candidate = held.get(index);
          if (candidate.key().equals(key) && candidate.value().equals(java.util.Optional.ofNullable(value))) {
            return held.remove(index).watermark();
          }
        }
        var event = events.poll(50, TimeUnit.MILLISECONDS);
        if (event != null) held.add(event);
      } while (System.nanoTime() < stop);
      throw new AssertionError("missing event for " + key);
    }
  }
}
