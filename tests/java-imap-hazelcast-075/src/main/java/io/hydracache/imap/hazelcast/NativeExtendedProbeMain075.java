package io.hydracache.imap.hazelcast;

import io.hydracache.imap.semantic.BytesValue;
import java.util.*;
import java.util.concurrent.TimeUnit;
import static io.hydracache.imap.hazelcast.NativeIMapObservation075.*;
import static io.hydracache.imap.hazelcast.NativeIMapProfile075.Operation.*;

/** Fixed packaged native probe; output is independently checked by the development tooling. */
public final class NativeExtendedProbeMain075 {
  private NativeExtendedProbeMain075() {}
  public static void main(String[] args) throws Exception {
    if (args.length != 0) throw new IllegalArgumentException("no native probe options accepted");
    try (var fixture = new HazelcastLoopback075()) { System.out.println(observe(fixture)); }
  }
  public static String observe(HazelcastLoopback075 fixture) throws Exception {
    var a = BytesValue.fromHex("00ff"); var b = BytesValue.fromHex("61"); var value = BytesValue.fromHex("0080ff");
    try (var adapter = new NativeIMapAdapter075(fixture.client(), "imap075-extended-" + UUID.randomUUID(), NativeIMapProfile075.exclusive())) {
      var first = adapter.execute(new Request(PUT_DETAILED, List.of(), List.of(new Entry(a, value)), Optional.empty(), 64));
      var second = adapter.execute(new Request(PUT_DETAILED, List.of(), List.of(new Entry(a, b), new Entry(b, value)), Optional.empty(), 64));
      var projections = NativeIMapProjection075.detailed(adapter.profile(), second);
      var events = await(adapter, 3);
      var view = ((ViewsReturn) adapter.execute(new Request(ENTRY_VIEWS, List.of(a), List.of(), Optional.empty(), 64)).rawReturn()).entries().get(0);
      var scan = (ViewsReturn) adapter.execute(Request.empty(SCAN)).rawReturn();
      var size = (NumberReturn) adapter.execute(Request.empty(SIZE)).rawReturn();
      var nonempty = (BooleanReturn) adapter.execute(Request.empty(IS_EMPTY)).rawReturn();
      var contains = (BooleanReturn) adapter.execute(new Request(CONTAINS_ENTRY, List.of(a), List.of(), Optional.of(b), 64)).rawReturn();
      var fallback = (ValueReturn) adapter.execute(new Request(GET_OR_DEFAULT, List.of(BytesValue.fromHex("62")), List.of(), Optional.of(value), 64)).rawReturn();
      var digest = adapter.snapshot().digest();
      var removed = (PointReturns) adapter.execute(new Request(REMOVE_DETAILED, List.of(a, BytesValue.fromHex("62")), List.of(), Optional.empty(), 64)).rawReturn(); await(adapter, 1);
      var aggregate = adapter.execute(new Request(PUT_ALL_DIAGNOSTIC, List.of(), List.of(new Entry(b, b)), Optional.empty(), 64)); await(adapter, 1);
      adapter.execute(Request.empty(CLEAR)); var clear = await(adapter, 1).get(0);
      var kinds = projections.stream().map(item -> "\"" + item.kind() + "\"").toList();
      return "{\"schema\":\"imap075-native-observation-v1\",\"profile\":\"" + NativeIMapProfile075.ID
          + "\",\"observed_fields\":[\"raw_return\",\"final_digest\",\"native_events\"],\"artifact_sha256\":\"" + HazelcastArtifactPin.verifyLoadedArtifact()
          + "\",\"point_calls\":" + (first.nativeCalls().orElseThrow() + second.nativeCalls().orElseThrow())
          + ",\"second_put_kinds\":[" + String.join(",", kinds) + "],\"second_put_previous\":[\"" + projections.get(0).previous().orElseThrow().hex()
          + "\",null],\"size\":" + size.value() + ",\"is_empty\":" + nonempty.value() + ",\"contains_entry\":" + contains.value()
          + ",\"fallback\":\"" + fallback.value().orElseThrow().hex() + "\",\"scan_count\":" + scan.entries().size()
          + ",\"native_version\":" + view.nativeVersion() + ",\"native_ttl_millis\":" + view.nativeTtlMillis()
          + ",\"native_event_count\":" + events.size() + ",\"snapshot_digest\":\"" + digest + "\",\"removed_previous\":[\""
          + removed.items().get(0).previous().orElseThrow().hex() + "\",null],\"put_all_origin\":\"" + aggregate.origin().name().toLowerCase(Locale.ROOT)
          + "\",\"clear_event\":\"" + clear.kind().name().toLowerCase(Locale.ROOT) + "\",\"clear_affected\":" + clear.affectedEntries()
          + ",\"final_digest\":\"" + adapter.snapshot().digest() + "\"}";
    }
  }
  static List<NativeEvent> await(NativeIMapAdapter075 adapter, int count) throws Exception {
    var events = new ArrayList<NativeEvent>(); long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
    do { events.addAll(adapter.drainEvents()); if (events.size() >= count) break; Thread.sleep(2); } while (System.nanoTime() < deadline);
    if (events.size() != count) throw new IllegalStateException("native callback completeness"); return List.copyOf(events);
  }
}
