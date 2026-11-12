package io.hydracache.imap.semantic;

import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;

/** Bounded, typed development grammar v2. No production protocol or durable IDs are assigned. */
public record ExtendedSemanticScenario(int version, long seed, List<Step> steps) {
  public static final int VERSION = 2;
  public static final int MAX_ITEMS = 64;
  public static final int MAX_STEPS = 256;
  public enum Op {
    PUT, CAPTURE_VERSION, REPLACE_IF_VERSION, REMOVE_IF_VERSION, SET_TTL_IF_VERSION,
    BULK_REPLACE, BULK_REMOVE, BULK_SET_TTL, QUERY_OUTCOMES, RETIRE_OUTCOMES,
    LISTENER_RESYNC, HOLD_LISTENER_SNAPSHOT, CLOSE_LISTENER_SNAPSHOT, CHECKPOINT_RESTORE,
    ADVANCE, EVICT, CLEAR, DESTROY, SIGNAL_GAP, RESOURCES, RESERVE_PUT, DISPATCH_PUT, CANCEL_RESERVED
  }
  public ExtendedSemanticScenario {
    Objects.requireNonNull(steps);
    if (version != VERSION || steps.size() > MAX_STEPS) throw new IllegalArgumentException("scenario version/step bound");
    long bytes = 0;
    var ids = new HashSet<String>();
    var keys = new HashSet<BytesValue>();
    var labels = new HashSet<String>();
    for (Step step : steps) {
      Objects.requireNonNull(step);
      if (!ids.add(step.id())) throw new IllegalArgumentException("duplicate step id");
      bytes += step.id().getBytes(StandardCharsets.UTF_8).length + 64;
      for (Entry entry : step.entries()) {
        keys.add(entry.key()); bytes += entry.key().size() + entry.value().map(BytesValue::size).orElse(0) + 128;
        if (step.operation() == Op.CAPTURE_VERSION) {
          if (!labels.add(entry.versionRef().orElseThrow()) || labels.size() > MAX_ITEMS) throw new IllegalArgumentException("token label bound/duplicate");
        } else if (entry.versionRef().isPresent() && !labels.contains(entry.versionRef().orElseThrow())) throw new IllegalArgumentException("unknown token label");
      }
      for (Request request : step.requests()) bytes += request.client().getBytes(StandardCharsets.UTF_8).length + 16;
      if (bytes > 512 * 1024 || keys.size() > MAX_ITEMS) throw new IllegalArgumentException("scenario payload/key bound");
    }
    steps = List.copyOf(steps);
  }
  public record Request(String client, long sequence) {
    public Request { text(client); if (sequence <= 0) throw new IllegalArgumentException("request sequence"); }
  }
  public record Entry(BytesValue key, Optional<BytesValue> value, Optional<String> versionRef, TtlDirective ttl) {
    public Entry {
      Objects.requireNonNull(key); Objects.requireNonNull(value); Objects.requireNonNull(versionRef); Objects.requireNonNull(ttl);
      if (key.size() > 1024 || value.map(BytesValue::size).orElse(0) > 4096) throw new IllegalArgumentException("entry bound");
      versionRef.ifPresent(ExtendedSemanticScenario::text);
    }
  }
  public record Limits(int maximumItems, long maximumBytes) {
    public Limits { if (maximumItems <= 0 || maximumItems > MAX_ITEMS || maximumBytes <= 0 || maximumBytes > 1024 * 1024) throw new IllegalArgumentException("snapshot bounds"); }
  }
  public record Step(String id, Op operation, List<Entry> entries, List<Request> requests,
      Optional<Limits> limits, long ticks, boolean loseResponse) {
    public Step {
      text(id); Objects.requireNonNull(operation); Objects.requireNonNull(entries); Objects.requireNonNull(requests); Objects.requireNonNull(limits);
      if (entries.size() > MAX_ITEMS || requests.size() > MAX_ITEMS) throw new IllegalArgumentException("step item bound");
      entries = List.copyOf(entries); requests = List.copyOf(requests);
      boolean point = switch (operation) { case PUT, RESERVE_PUT, DISPATCH_PUT, CAPTURE_VERSION, REPLACE_IF_VERSION, REMOVE_IF_VERSION, SET_TTL_IF_VERSION, EVICT -> true; default -> false; };
      boolean bulk = switch (operation) { case BULK_REPLACE, BULK_REMOVE, BULK_SET_TTL -> true; default -> false; };
      boolean mutation = mutation(operation);
      boolean snapshot = operation == Op.LISTENER_RESYNC || operation == Op.HOLD_LISTENER_SNAPSHOT;
      if ((point && entries.size() != 1) || (bulk && entries.isEmpty()) || (!point && !bulk && !entries.isEmpty())
          || (mutation && requests.size() != 1)
          || ((operation == Op.RETIRE_OUTCOMES || operation == Op.CANCEL_RESERVED) && requests.size() != 1)
          || (operation == Op.QUERY_OUTCOMES && requests.isEmpty())
          || (!mutation && operation != Op.RETIRE_OUTCOMES && operation != Op.QUERY_OUTCOMES && operation != Op.CANCEL_RESERVED && !requests.isEmpty())
          || snapshot != limits.isPresent() || (operation == Op.ADVANCE ? ticks <= 0 : ticks != 0)
          || (loseResponse && (!mutation || operation == Op.RESERVE_PUT))) throw new IllegalArgumentException("step shape");
      for (Entry entry : entries) {
        boolean value = operation == Op.PUT || operation == Op.RESERVE_PUT || operation == Op.DISPATCH_PUT || operation == Op.REPLACE_IF_VERSION || operation == Op.BULK_REPLACE;
        boolean token = operation != Op.PUT && operation != Op.RESERVE_PUT && operation != Op.DISPATCH_PUT && operation != Op.EVICT;
        if (value != entry.value().isPresent() || token != entry.versionRef().isPresent()) throw new IllegalArgumentException("entry shape");
      }
    }
  }
  public static boolean mutation(Op operation) {
    return switch (operation) { case PUT, RESERVE_PUT, DISPATCH_PUT, REPLACE_IF_VERSION, REMOVE_IF_VERSION, SET_TTL_IF_VERSION, BULK_REPLACE, BULK_REMOVE, BULK_SET_TTL, EVICT, CLEAR, DESTROY -> true; default -> false; };
  }
  public record Row(BytesValue key, BytesValue value, TtlState ttl) { public Row { Objects.requireNonNull(key); Objects.requireNonNull(value); Objects.requireNonNull(ttl); payload(key, Optional.of(value)); } }
  /** Numeric token gaps are not compared; equality, freshness and instance scope are. */
  public record Item(int index, BytesValue key, String kind, Optional<BytesValue> value, TtlState ttl) {
    public Item { if (index < 0) throw new IllegalArgumentException("item index"); Objects.requireNonNull(key); text(kind); Objects.requireNonNull(value); Objects.requireNonNull(ttl); payload(key, value); }
  }
  public record Status(Request identity, String kind, Optional<Result> result) {
    public Status {
      Objects.requireNonNull(identity); text(kind); Objects.requireNonNull(result);
      if (result.isPresent() && !result.orElseThrow().statuses().isEmpty()) throw new IllegalArgumentException("nested outcome status");
    }
  }
  public record Result(String kind, Optional<BytesValue> value, TtlState ttl, List<Item> items, List<Status> statuses, long number) {
    public Result {
      text(kind); Objects.requireNonNull(value); Objects.requireNonNull(ttl); payload(BytesValue.copyOf(new byte[0]), value);
      items = bounded(items, MAX_ITEMS); statuses = bounded(statuses, MAX_ITEMS);
      if (number < 0) throw new IllegalArgumentException("result number");
    }
    public static Result simple(String kind) { return new Result(kind, Optional.empty(), TtlState.absent(), List.of(), List.of(), 0); }
    public static Result numbered(String kind, long number) { return new Result(kind, Optional.empty(), TtlState.absent(), List.of(), List.of(), number); }
  }
  public record Event(String kind, BytesValue key, Optional<BytesValue> oldValue, Optional<BytesValue> value, long watermark) {
    public Event { text(kind); Objects.requireNonNull(key); Objects.requireNonNull(oldValue); Objects.requireNonNull(value); payload(key, oldValue); payload(key, value); if (watermark <= 0) throw new IllegalArgumentException("event watermark"); }
  }
  public record Owners(long outcomes, long pending, long listeners, boolean snapshotPayload) {
    public Owners { if (outcomes < 0 || pending < 0 || listeners < 0) throw new IllegalArgumentException("negative ownership"); }
  }
  public record Observation(Result result, List<Row> snapshot, List<Event> events, Map<String, Boolean> tokenRelations, Owners owners, long tick) {
    public Observation {
      Objects.requireNonNull(result); snapshot = bounded(snapshot, MAX_ITEMS); events = bounded(events, MAX_ITEMS + 1);
      Objects.requireNonNull(tokenRelations); if (tokenRelations.size() > MAX_ITEMS * MAX_ITEMS) throw new IllegalArgumentException("token relation bound");
      for (String key : tokenRelations.keySet()) if (key.getBytes(StandardCharsets.UTF_8).length > 193) throw new IllegalArgumentException("token relation byte bound");
      tokenRelations = Map.copyOf(tokenRelations); Objects.requireNonNull(owners); if (tick < 0) throw new IllegalArgumentException("negative tick");
    }
  }
  static void text(String value) {
    Objects.requireNonNull(value); if (value.isBlank() || value.getBytes(StandardCharsets.UTF_8).length > 96 || value.indexOf('=') >= 0) throw new IllegalArgumentException("identifier bound");
  }
  private static void payload(BytesValue key, Optional<BytesValue> value) { if (key.size() > 1024 || value.map(BytesValue::size).orElse(0) > 4096) throw new IllegalArgumentException("observation payload bound"); }
  private static <T> List<T> bounded(List<T> list, int maximum) { Objects.requireNonNull(list); if (list.size() > maximum) throw new IllegalArgumentException("observation item bound"); return List.copyOf(list); }
}
