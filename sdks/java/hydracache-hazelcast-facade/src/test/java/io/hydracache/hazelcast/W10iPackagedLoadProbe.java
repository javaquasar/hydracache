package io.hydracache.hazelcast;

import io.hydracache.client.hc2.BatchItemOutcome;
import io.hydracache.client.hc2.BatchItemReceipt;
import io.hydracache.client.hc2.BatchOperation;
import io.hydracache.client.hc2.CacheEvent;
import io.hydracache.client.hc2.CacheEventListener;
import io.hydracache.client.hc2.ErrorCode;
import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.HydraCacheException;
import io.hydracache.client.hc2.RequestOptions;
import io.hydracache.client.hc2.SubscriptionGap;
import java.io.BufferedWriter;
import java.io.FileOutputStream;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.List;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.CompletionException;
import java.util.concurrent.TimeUnit;

public final class W10iPackagedLoadProbe {
  private static final RequestOptions OPTIONS = RequestOptions.withTimeout(Duration.ofSeconds(2));

  public static void main(String[] args) throws Exception {
    W10gPackagedFacadeProbe.requireJar(HydraCacheClient.class);
    var root = Path.of(args[1]);
    try (var client = W10gPackagedFacadeProbe.connect(root, args[2])) {
      switch (args[0]) {
        case "load" -> load(client, root, Long.parseLong(args[3]));
        case "watch" -> watch(client, root, args);
        default -> throw new AssertionError("unknown packaged workload mode");
      }
    }
  }

  private static void load(HydraCacheClient client, Path root, long origin) throws Exception {
    var stop = root.resolve("packaged-load.stop");
    long alignedAt = System.nanoTime();
    long offset = System.currentTimeMillis() - origin;
    require(offset >= 0, "Java probe started before the campaign origin");
    int seq = 0;
    try (var plan = Files.newBufferedReader(root.resolve("java-load.plan"), StandardCharsets.UTF_8);
         var file = new FileOutputStream(root.resolve("java-load.tsv").toFile());
         var ledger = new BufferedWriter(new OutputStreamWriter(file, StandardCharsets.UTF_8))) {
      String row;
      while (!Files.exists(stop) && (row = plan.readLine()) != null) {
        var pair = row.split("\t", 3);
        var a = HexFormat.of().parseHex(pair[0]);
        var b = HexFormat.of().parseHex(pair[1]);
        var batch = List.<BatchOperation>of(new BatchOperation.Put(a, bytes("seed"), null),
            new BatchOperation.CompareAndSet(a, bytes("stale"), bytes("updated"), null),
            new BatchOperation.CompareAndSet(a, bytes("seed"), bytes("updated"), null),
            new BatchOperation.Get(a), new BatchOperation.Put(b, bytes("seed"), null),
            new BatchOperation.Delete(b), new BatchOperation.Get(b));
        ledger.write("submit\t" + seq + "\t" + elapsed(offset, alignedAt) + "\t" + pair[0] + "\t" + pair[1] + "\n");
        ledger.flush();
        String encoded;
        try {
          var outcomes = client.batchOutcomes(batch, OPTIONS).join();
          require(outcomes.size() == 7, "wrong Java load outcome count");
          var fields = new String[7];
          for (int index = 0; index < fields.length; ++index) fields[index] = encode(outcomes.get(index), index);
          encoded = String.join("\t", fields);
        } catch (CompletionException error) {
          if (!unavailable(error.getCause())) throw error;
          encoded = "u\tu\tu\tu\tu\tu\tu";
        }
        ledger.write("outcome\t" + seq + "\t" + elapsed(offset, alignedAt) + "\t" + encoded + "\n");
        ledger.flush();
        if (seq == 0) Files.createFile(root.resolve("java-load.ready"));
        ++seq;
        Thread.sleep(200);
      }
      require(Files.exists(stop), "Java load exhausted its key plan");
      file.getFD().sync();
    }
    System.out.printf("W10i Java packaged action=load rounds=%d%n", seq);
  }

  private static long elapsed(long offset, long alignedAt) {
    return offset + TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - alignedAt);
  }

  private static String encode(BatchItemOutcome row, int index) {
    require(row.itemId() == index + 1, "wrong Java load item identity");
    if (row.receipt().isEmpty()) {
      require(row.error().isPresent() && unavailable(row.error().orElseThrow()), "missing Java load receipt");
      return "u";
    }
    var receipt = row.receipt().orElseThrow();
    if (receipt.state() != BatchItemReceipt.State.CONFIRMED) {
      require(row.error().isPresent(), "unconfirmed Java result");
      return receipt.state() == BatchItemReceipt.State.NOT_APPLIED ? "n" : "u";
    }
    var version = receipt.applied().orElseThrow();
    var result = row.result().orElseThrow();
    String state;
    if (index == 3 || index == 6) {
      require(result.mutation().isEmpty(), "mutation in Java readback");
      state = result.value().map(value -> switch (new String(value.value(), StandardCharsets.UTF_8)) {
        case "seed" -> "s";
        case "updated" -> "v";
        default -> throw new AssertionError("unexpected Java load readback");
      }).orElse("0");
    } else {
      state = result.mutation().orElseThrow().applied() ? "a" : "r";
    }
    return state + ":" + receipt.partitionId() + ":" + version.epoch() + ":" + version.version();
  }

  private static void watch(HydraCacheClient client, Path root, String[] args) throws Exception {
    String label = args[3];
    byte[] key = HexFormat.of().parseHex(args[4]);
    long resume = Long.parseLong(args[5]);
    var events = new ArrayBlockingQueue<Object>(8);
    var listener = new CacheEventListener() {
      @Override public void onEvent(CacheEvent event) { events.add(event); }
      @Override public void onGap(long id, long after) { events.add(after); }
      @Override public void onGap(long id, long after, SubscriptionGap gap) { events.add(new Gap(id, after, gap)); }
    };
    try (var subscription = client.subscribe(key, resume, true, listener).join()) {
      if (resume > 0) {
        var first = events.poll(10, TimeUnit.SECONDS);
        require(first instanceof Gap gap && gap.subscription() == subscription.id() && gap.after() == resume
            && gap.detail().reason() == SubscriptionGap.Reason.RESUME_UNBOUND
            && gap.detail().partitions().isEmpty(), "missing Java resume boundary: " + first);
      }
      var current = client.get(key, OPTIONS).join().map(value -> value.value()).orElse(null);
      require(Arrays.equals(current, args[6].equals("-") ? null : bytes(args[6])), "Java subscriber repair readback differed");
      Files.createFile(root.resolve("java-" + label + ".ready"));
      var received = events.poll(10, TimeUnit.SECONDS);
      require(received instanceof CacheEvent, "missing fresh Java event: " + received);
      var event = (CacheEvent) received;
      require(event.subscriptionId() == subscription.id() && !event.removed()
          && Arrays.equals(event.key(), key) && Arrays.equals(event.value(), bytes(args[7]))
          && event.watermark() > resume, "invalid fresh Java event");
      var marker = Files.writeString(root.resolve("java-" + label + ".tmp"), Long.toString(event.watermark()));
      Files.move(marker, root.resolve("java-" + label + ".seen"));
      if (args[8].equals("hold")) {
        var release = root.resolve("watch-" + label + ".release");
        long deadline = System.nanoTime() + Duration.ofSeconds(55).toNanos();
        while (!Files.exists(release)) {
          require(System.nanoTime() < deadline, "Java subscription was not released after the fault");
          Thread.sleep(20);
        }
      }
      System.out.printf("W10i Java packaged action=watch stage=%s resumed=%d seen=%d%n", label, resume, event.watermark());
    }
  }

  private record Gap(long subscription, long after, SubscriptionGap detail) {}

  private static boolean unavailable(Throwable failure) {
    return failure instanceof HydraCacheException error
        && (error.code() == ErrorCode.UNAVAILABLE || error.code() == ErrorCode.DEADLINE_EXCEEDED);
  }

  private static byte[] bytes(String value) { return value.getBytes(StandardCharsets.UTF_8); }

  private static void require(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
