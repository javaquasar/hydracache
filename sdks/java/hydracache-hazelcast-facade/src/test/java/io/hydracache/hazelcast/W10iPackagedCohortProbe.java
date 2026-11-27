package io.hydracache.hazelcast;

import io.hydracache.client.hc2.BatchItemReceipt;
import io.hydracache.client.hc2.BatchOperation;
import io.hydracache.client.hc2.HydraCacheClient;
import io.hydracache.client.hc2.RequestOptions;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import static io.hydracache.hazelcast.W10gPackagedFacadeProbe.awaitBatch;
import static io.hydracache.hazelcast.W10gPackagedFacadeProbe.connect;
import static io.hydracache.hazelcast.W10gPackagedFacadeProbe.requireJar;

/** Cross-SDK acknowledgement and readback on either side of a W10i process kill. */
public final class W10iPackagedCohortProbe {
  private static final Duration PHASE_TIMEOUT = Duration.ofSeconds(20);
  private static final RequestOptions REQUEST = RequestOptions.withTimeout(Duration.ofSeconds(3));
  private static final int KEYS_PER_PHASE = 4;
  private static final int PHASES = 3;

  public static void main(String[] args) throws Exception {
    requireJar(HydraMap.class);
    requireJar(HydraCacheClient.class);
    if (args.length == 4 && args[0].equals("check-file")) {
      verifyPhysical(Path.of(args[1]), args[2], args[3]);
      return;
    }
    require(args.length == 6 + PHASES * KEYS_PER_PHASE, "wrong cohort size");
    Path root = Path.of(args[0]);
    String hot = args[4];
    boolean wrongJava = args[5].equals("wrong-java");
    require(wrongJava || args[5].equals("correct"), "unknown cohort mode");
    var expected = new LinkedHashMap<String, String>();
    try (var writer = connect(root, args[3]); var reader = connect(root, args[2])) {
      var sender = map(writer);
      var observer = map(reader);
      Files.createFile(root.resolve("cohort-ready"));
      for (int phase = 0; phase < PHASES; ++phase) {
        awaitPhase(root.resolve("cohort-start-" + phase));
        List<String> keys = Arrays.asList(args).subList(
            6 + phase * KEYS_PER_PHASE, 6 + (phase + 1) * KEYS_PER_PHASE);
        var rust = new LinkedHashMap<String, String>();
        rust.put(keys.get(0), "rust:" + keys.get(0));
        rust.put(keys.get(1), "rust:" + keys.get(1));
        verifyValues(observer, rust);
        expected.putAll(rust);

        var java = new LinkedHashMap<String, String>();
        java.put(keys.get(2), wrongJava ? "intentional-wrong-java-value" : "java:" + keys.get(2));
        java.put(keys.get(3), "java:" + keys.get(3));
        if (phase == 0) java.put(hot, "java:" + hot);
        var options = phase == 0
            ? new RequestOptions(REQUEST.timeout(), "java-w10i-cohort".getBytes(StandardCharsets.UTF_8))
            : REQUEST;
        var outcomes = sender.putAll(java, options);
        require(outcomes.stream().map(MapBatchItem::key).toList().equals(List.copyOf(java.keySet())),
            "Java batch receipt keys changed");
        for (var item : outcomes) {
          var receipt = item.outcome().receipt().orElseThrow();
          require(receipt.state() == BatchItemReceipt.State.CONFIRMED
              && receipt.applied().isPresent()
              && item.outcome().result().orElseThrow().mutation().orElseThrow().applied(),
              "Java did not confirm " + item.key() + ": " + receipt);
        }
        expected.putAll(java);
        verifyValues(observer, expected);
        if (phase == PHASES - 1) {
          try (var returned = connect(root, args[1])) {
            verifyValues(map(returned), expected);
          }
        }
        System.out.printf("W10i Java cohort phase=%d confirmed=%d readback=%d%n",
            phase, outcomes.size(), expected.size());
        Files.createFile(root.resolve("cohort-done-" + phase));
      }
    }
  }

  private static HydraMap<String, String> map(HydraCacheClient client) {
    return new HydraCacheInstance(client, REQUEST.timeout())
        .getMap("w10i-sdk-cohort", HydraCodec.utf8(), HydraCodec.utf8());
  }

  private static void verifyPhysical(Path root, String endpoint, String file) throws Exception {
    Path manifest = root.resolve(file);
    require(Files.size(manifest) <= 128 * 1024, "readback manifest too large");
    var rows = Files.readAllLines(manifest);
    require(!rows.isEmpty() && rows.size() <= 14, "invalid readback count");
    var entries = rows.stream().map(row -> row.split("\t", 2)).toList();
    var operations = entries.stream().<BatchOperation>map(entry -> {
      require(entry.length == 2, "readback row lacks its value");
      return new BatchOperation.Get(HexFormat.of().parseHex(entry[0]));
    }).toList();
    try (var client = connect(root, endpoint)) {
      var outcomes = client.batchOutcomes(operations, REQUEST).join();
      require(outcomes.size() == entries.size(), "wrong readback outcome count");
      for (int index = 0; index < entries.size(); ++index) {
        var outcome = outcomes.get(index);
        var receipt = outcome.receipt().orElseThrow();
        require(receipt.state() == BatchItemReceipt.State.CONFIRMED
            && receipt.applied().orElseThrow().version() > 0, "unconfirmed physical key");
        var value = outcome.result().orElseThrow().value().orElseThrow().value();
        require(Arrays.equals(value, entries.get(index)[1].getBytes(StandardCharsets.UTF_8)),
            "wrong physical value at row " + index);
      }
    }
    System.out.printf("W10i Java cohort action=check-file confirmed=%d%n", entries.size());
  }

  private static void verifyValues(HydraMap<String, String> map, Map<String, String> expected)
      throws InterruptedException {
    awaitBatch(map, List.copyOf(expected.keySet()), rows -> {
      for (var row : rows) {
        row.value().ifPresent(value -> require(value.equals(expected.get(row.key())),
            "wrong value for " + row.key()));
      }
      return rows.stream().allMatch(row -> row.value().isPresent());
    });
  }

  private static void awaitPhase(Path marker) throws InterruptedException {
    long stop = System.nanoTime() + PHASE_TIMEOUT.toNanos();
    while (!Files.exists(marker)) {
      require(System.nanoTime() < stop, "phase did not start: " + marker.getFileName());
      Thread.sleep(20);
    }
  }

  private static void require(boolean condition, String message) {
    if (!condition) throw new AssertionError(message);
  }
}
