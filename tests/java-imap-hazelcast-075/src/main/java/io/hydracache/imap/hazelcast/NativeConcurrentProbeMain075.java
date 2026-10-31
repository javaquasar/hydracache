package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.util.*;
import static io.hydracache.imap.hazelcast.NativeConcurrentHistory075.*;

/** Bounded raw histories for independent replay/checking. UUIDs are diagnostic identities. */
public final class NativeConcurrentProbeMain075 {
  private NativeConcurrentProbeMain075() {}
  public static void main(String[] args) throws Exception {
    if (args.length != 0) throw new IllegalArgumentException("no native history probe options");
    try (var fixture = new HazelcastLoopback075()) { System.out.println(observe(fixture)); }
  }
  public static String observe(HazelcastLoopback075 fixture) throws Exception {
    var histories = new ArrayList<String>();
    for (long seed : new long[]{117, 118}) histories.add(encode(NativeConcurrentWorker075.qualified(fixture, NativeConcurrentScenario075.seeded(seed), 8)));
    return "{\"schema\":\"imap075-native-histories-v1\",\"scope\":\"eternal_fixed_keys_two_clients_single_member\",\"artifact_sha256\":\""
        + HazelcastArtifactPin.verifyLoadedArtifact() + "\",\"histories\":[" + String.join(",", histories) + "]}";
  }
  static String nullable(Optional<?> value) { return value.map(v -> v instanceof BytesValue bytes ? "\"" + bytes.hex() + "\"" : v.toString()).orElse("null"); }
  public static String encode(NativeConcurrentWorker075.Run run) {
    var history = run.history(); var report = NativeConcurrentOracle075.check(history, 100_000);
    var commands = history.program().commands().stream().map(c -> "{\"id\":" + c.id() + ",\"worker\":" + c.worker() + ",\"position\":" + c.position()
        + ",\"operation\":\"" + c.operation().name().toLowerCase(Locale.ROOT) + "\",\"key\":\"" + c.key().hex() + "\",\"value\":" + nullable(c.value()) + ",\"expected\":" + nullable(c.expected()) + "}").toList();
    var raw = history.raw().stream().map(e -> {
      String result = e instanceof Return end ? ",\"value\":" + nullable(end.result().value()) + ",\"boolean\":" + nullable(end.result().booleanValue()) : ",\"value\":null,\"boolean\":null";
      return "{\"phase\":\"" + (e instanceof Invoke ? "invoke" : "return") + "\",\"stamp\":" + e.stamp() + ",\"call_id\":" + e.callId() + ",\"worker\":" + e.worker() + result + "}";
    }).toList();
    var entries = history.finalState().entries().stream().map(e -> "{\"key\":\"" + e.key().hex() + "\",\"value\":\"" + e.value().hex() + "\"}").toList();
    var events = run.nativeEvents().stream().map(e -> "{\"kind\":\"" + e.kind().name().toLowerCase(Locale.ROOT) + "\",\"key\":" + nullable(e.key())
        + ",\"old_value\":" + nullable(e.oldValue()) + ",\"value\":" + nullable(e.value()) + "}").toList();
    var clients = run.clients().stream().map(id -> "\"" + id + "\"").toList();
    return "{\"seed\":" + history.program().seed() + ",\"attempts\":" + run.attempts() + ",\"member_uuid\":\"" + run.member() + "\",\"clients\":[" + String.join(",", clients)
        + "],\"program\":[" + String.join(",", commands) + "],\"raw\":[" + String.join(",", raw) + "],\"raw_digest\":\"" + history.rawDigest()
        + "\",\"final_entries\":[" + String.join(",", entries) + "],\"final_digest\":\"" + history.finalState().digest() + "\",\"overlap\":" + history.hasOverlap()
        + ",\"verdict\":\"" + report.verdict().name().toLowerCase(Locale.ROOT) + "\",\"work\":" + report.work() + ",\"witness\":" + report.witness()
        + ",\"native_events\":[" + String.join(",", events) + "]}";
  }
}
