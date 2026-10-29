package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.util.*;
import java.util.concurrent.TimeUnit;

/** Controlled native queue-loss observation, not production snapshot/listener fencing. */
public final class NativeListenerRecoveryProbeMain075 {
  private NativeListenerRecoveryProbeMain075() {}
  public static void main(String[] args) throws Exception {
    if (args.length != 0) throw new IllegalArgumentException("no listener recovery probe options");
    try (var cluster = new HazelcastLoopback075()) { System.out.println(observe(cluster)); }
  }
  public static String observe(HazelcastLoopback075 cluster) throws Exception {
    String name = "imap075-recovery-probe-" + UUID.randomUUID(); var key = BytesValue.fromHex("61");
    var fixture = new NativeListenerRecovery075(cluster.client(), name, 2);
    try {
      for (int i = 0; i < 8; i++) fixture.put(key, BytesValue.copyOf(new byte[]{(byte)i}));
      await(fixture, 8); var gap = fixture.reason().orElseThrow(); int callbacks = fixture.observedCallbacks();
      var cut = fixture.recover(NativeListenerRecovery075.Bounds.smoke(), new NativeListenerRecovery075.Cancellation());
      fixture.put(key, BytesValue.fromHex("08")); await(fixture, 1); var events = fixture.drainEvents();
      if (events.size() != 1) throw new IllegalStateException("post-recovery callback completeness");
      var event = events.get(0); var healthy = fixture.state(); fixture.close(); fixture.close();
      byte[] stored = cluster.client().<byte[],byte[]>getMap(name).get(key.copy());
      return "{\"schema\":\"imap075-native-recovery-v1\",\"profile\":\"controlled_unique_point_writers_queue_overflow\",\"artifact_sha256\":\""
          + HazelcastArtifactPin.verifyLoadedArtifact() + "\",\"accepted_writes\":8,\"callbacks_observed\":" + callbacks + ",\"correlated_writes\":" + cut.correlatedWrites()
          + ",\"queue_capacity\":2,\"gap_before\":\"" + gap.name().toLowerCase(Locale.ROOT) + "\",\"old_generation\":" + cut.oldGeneration() + ",\"new_generation\":" + cut.newGeneration()
          + ",\"baseline_key\":\"" + cut.baseline().entries().get(0).key().hex() + "\",\"baseline_value\":\"" + cut.baseline().entries().get(0).value().hex()
          + "\",\"baseline_digest\":\"" + cut.baseline().digest() + "\",\"post_event_generation\":" + event.registrationGeneration()
          + ",\"post_event_kind\":\"" + event.kind().name().toLowerCase(Locale.ROOT) + "\",\"post_event_key\":\"" + event.key().orElseThrow().hex()
          + "\",\"post_event_old_value\":\"" + event.oldValue().orElseThrow().hex() + "\",\"post_event_value\":\"" + event.value().orElseThrow().hex()
          + "\",\"state_after_recovery\":\"" + healthy.name().toLowerCase(Locale.ROOT) + "\",\"state_after_close\":\"" + fixture.state().name().toLowerCase(Locale.ROOT)
          + "\",\"removed_subscriptions\":" + fixture.removedSubscriptions() + ",\"client_alive_after_close\":" + cluster.client().getLifecycleService().isRunning()
          + ",\"map_value_after_close\":\"" + BytesValue.copyOf(stored).hex() + "\"}";
    } finally { fixture.close(); }
  }
  private static void await(NativeListenerRecovery075 fixture, int count) throws InterruptedException {
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
    while (fixture.observedCallbacks() < count && System.nanoTime() < deadline) Thread.sleep(1);
    if (fixture.observedCallbacks() != count) throw new IllegalStateException("native callbacks incomplete");
  }
}
