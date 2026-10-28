package io.hydracache.imap.semantic;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.OptionalLong;
import java.util.Set;
import java.util.TreeMap;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.LongSupplier;

/** One local command service over the existing bounded map, with no network dispatch. */
public final class UnifiedIMapService {
  private static final AtomicLong NEXT_MAP = new AtomicLong(1);
  private static final BytesValue EMPTY = BytesValue.copyOf(new byte[0]);
  private static final Set<String> MUTATIONS = Set.of("put", "delete", "conditional_replace", "conditional_remove",
      "put_if_absent", "replace_if_present", "get_and_remove", "get_and_put", "put_all_detailed", "remove_all_detailed",
      "set_ttl", "clear_detailed", "destroy", "evict", "evict_all_detailed", "set", "try_put", "try_remove",
      "replace_all_detailed", "replace_if_version", "remove_if_version", "set_ttl_if_version");

  public record VersionToken(long namespaceGeneration, long mapId, long version) {
    public VersionToken {
      if (namespaceGeneration <= 0 || mapId <= 0 || version <= 0) invalid("invalid version token");
    }
  }
  public record Item(int index, BytesValue key, String kind, Optional<BytesValue> value,
      OptionalLong version, OptionalLong remainingTtl) {}
  public record Result(String kind, Optional<BytesValue> value, OptionalLong version,
      OptionalLong remainingTtl, List<Item> items, long number, boolean flag,
      Optional<VersionToken> token, Optional<BoundedIMapApi.Cursor> cursor) {
    public Result { items = List.copyOf(items); }
    public Result(String kind, Optional<BytesValue> value, OptionalLong version,
        OptionalLong remainingTtl, List<Item> items, long number, boolean flag,
        Optional<VersionToken> token) {
      this(kind, value, version, remainingTtl, items, number, flag, token, Optional.empty());
    }
  }
  public record SnapshotEntry(BytesValue key, BytesValue value, long version,
      OptionalLong remainingTtl) {}
  public record Statistics(long liveEntries, long valueBytes, long retainedOutcomes,
      long listeners, long queuedEvents, long inflight, long rejections, long cancellations,
      long requests, long activeStreams, long activeCursors) {}
  public record Event(String kind, BytesValue key, Optional<BytesValue> oldValue,
      Optional<BytesValue> value, long version, long watermark) {}
  public record ListenerOptions(boolean includeValue, int maxValueBytes, int queueCapacity,
      Optional<BytesValue> exactKey, Set<String> kinds) {
    public ListenerOptions {
      exactKey = Objects.requireNonNull(exactKey, "exactKey");
      kinds = Set.copyOf(kinds);
      if (maxValueBytes < 0 || queueCapacity <= 0) invalid("invalid listener options");
      if (!Set.of("added", "updated", "removed", "expired", "evicted", "cleared", "destroyed")
          .containsAll(kinds)) invalid("unsupported listener event kind");
    }
    public static ListenerOptions metadata(int capacity) {
      return new ListenerOptions(false, 0, capacity, Optional.empty(), Set.of());
    }
  }
  public record OperationStatus(String kind, Optional<Result> result) {}

  /** Mutable request builder; execution snapshots its bounded immutable identity digest. */
  public static final class DomainCommand {
    public final String operation;
    public BytesValue key = EMPTY;
    public Optional<BytesValue> value = Optional.empty(), expected = Optional.empty();
    public List<BytesValue> keys = List.of();
    public List<BoundedIMapApi.ReplaceInput> entries = List.of();
    public Optional<BoundedIMapApi.MutationIdentity> identity = Optional.empty();
    public TtlDirective ttl = TtlDirective.eternal();
    public long now, deadline = Long.MAX_VALUE, listenerId;
    public Optional<VersionToken> token = Optional.empty();
    public Optional<BoundedIMapApi.Cursor> cursor = Optional.empty();
    public int limit = 256;
    public boolean includeValue, loseResponse, cancelled;
    public DomainCommand(String operation) { this.operation = Objects.requireNonNull(operation); }
    public DomainCommand key(BytesValue v) { key = Objects.requireNonNull(v); return this; }
    public DomainCommand value(BytesValue v) { value = Optional.of(v); return this; }
    public DomainCommand expected(BytesValue v) { expected = Optional.of(v); return this; }
    public DomainCommand keys(List<BytesValue> v) { keys = List.copyOf(v); return this; }
    public DomainCommand entries(List<BoundedIMapApi.ReplaceInput> v) { entries = List.copyOf(v); return this; }
    public DomainCommand identity(BoundedIMapApi.MutationIdentity v) { identity = Optional.of(v); return this; }
    public DomainCommand ttl(TtlDirective v) { ttl = Objects.requireNonNull(v); return this; }
    public DomainCommand now(long v) { now = v; return this; }
    public DomainCommand deadline(long v) { deadline = v; return this; }
    public DomainCommand token(VersionToken v) { token = Optional.of(v); return this; }
    public DomainCommand cursor(BoundedIMapApi.Cursor v) { cursor = Optional.of(v); return this; }
    public DomainCommand listenerId(long v) { listenerId = v; return this; }
    public DomainCommand limit(int v) { limit = v; return this; }
    public DomainCommand includeValue(boolean v) { includeValue = v; return this; }
    public DomainCommand loseResponse(boolean v) { loseResponse = v; return this; }
    public DomainCommand cancelled(boolean v) { cancelled = v; return this; }
    public DomainCommand copy() {
      DomainCommand result = new DomainCommand(operation);
      result.key = key; result.value = value; result.expected = expected; result.keys = List.copyOf(keys);
      result.entries = List.copyOf(entries); result.identity = identity; result.ttl = ttl;
      result.now = now; result.deadline = deadline; result.listenerId = listenerId;
      result.token = token; result.limit = limit; result.includeValue = includeValue;
      result.cursor = cursor;
      result.loseResponse = loseResponse; result.cancelled = cancelled; return result;
    }
  }

  private record Request(String operation, BytesValue key, Optional<BytesValue> value,
      Optional<BytesValue> expected, List<BytesValue> keys, List<BoundedIMapApi.ReplaceInput> entries,
      TtlDirective ttl, Optional<VersionToken> token) {}
  private static final class ListenerState {
    final ListenerOptions options;
    final ArrayDeque<Event> queue = new ArrayDeque<>();
    boolean gap, terminal, streamAttached;
    Runnable wake = () -> {};
    ListenerState(ListenerOptions options) { this.options = options; }
  }

  private final BoundedIMapApi map;
  private final BoundedIMapApi.Bounds bounds;
  private final long generation, mapId;
  private final TreeMap<Long, ListenerState> listeners = new TreeMap<>();
  private final Set<Runnable> pendingWake = new java.util.LinkedHashSet<>();
  private long now, watermark, requests, rejections, cancellations, activeStreams, activeCursors;

  public UnifiedIMapService(BoundedIMapApi.Bounds bounds, long namespaceGeneration) {
    this(new BoundedIMapApi(bounds), namespaceGeneration);
  }

  public UnifiedIMapService(BoundedIMapApi existing, long namespaceGeneration) {
    if (namespaceGeneration <= 0) invalid("namespace generation must be positive");
    map = Objects.requireNonNull(existing);
    this.bounds = map.serviceBounds();
    if (bounds.maxPageItems() > 256) invalid("page bound must not exceed 256");
    generation = namespaceGeneration;
    mapId = NEXT_MAP.getAndUpdate(value -> Math.incrementExact(value));
  }

  public Result execute(DomainCommand command) {
    try { synchronized (this) { return executeLocked(command); } }
    finally { wakeListeners(); }
  }

  private Result executeLocked(DomainCommand command) {
    Objects.requireNonNull(command, "command");
    requests++;
    BoundedIMapApi.MutationIdentity identity = null;
    boolean admitted = false;
    try {
      validate(command);
      if (isMutation(command.operation) && command.identity.isPresent()) {
        Object prior = map.recoverService(command.identity.orElseThrow(), request(command));
        if (prior instanceof Result result) return result;
      }
      advance(command.now);
      if (!isMutation(command.operation)) return read(command);
      identity = command.identity.orElseThrow(() -> domain(IMapDomainException.Kind.INVALID_ARGUMENT,
          "mutation identity required"));
      Request digest = request(command);
      Object prior = map.recoverService(identity, digest);
      if (prior instanceof Result result) return result;
      if (command.cancelled || command.now >= command.deadline) {
        map.abandonService(identity);
        if (command.cancelled) cancellations++;
        return simple("not_applied");
      }
      map.beginService(identity, digest);
      admitted = true;
      List<BoundedIMapApi.RetainedEntry> before = map.retainedEntries();
      Result result = mutate(command);
      if (!map.isDestroyed()) map.completeService(identity, digest, result);
      if (!Set.of("put_all_detailed", "replace_all_detailed", "remove_all_detailed")
          .contains(command.operation)) publishChanges(command.operation, before, map.retainedEntries());
      if (command.loseResponse) return simple("outcome_unknown");
      return result;
    } catch (RuntimeException failure) {
      if (admitted && identity != null) map.abandonService(identity);
      rejections++;
      throw IMapDomainException.translate(failure);
    }
  }

  public synchronized void beginOperation(DomainCommand command) {
    try {
      validate(command);
      if (!isMutation(command.operation)) invalid("only mutation identities can be pending");
      var identity = command.identity.orElseThrow(() -> domain(
          IMapDomainException.Kind.INVALID_ARGUMENT, "mutation identity required"));
      if (map.recoverService(identity, request(command)) != null) return;
      advance(command.now);
      map.beginService(identity, request(command));
    } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
  }

  public synchronized void abandonOperation(BoundedIMapApi.MutationIdentity identity) {
    map.abandonService(identity);
  }
  public Result executeReserved(DomainCommand command) {
    synchronized (this) {
      if (isMutation(command.operation)) {
        var identity = command.identity.orElseThrow(() -> domain(IMapDomainException.Kind.INVALID_ARGUMENT, "identity required"));
        if (!map.servicePending(identity) && map.recoverService(identity, request(command)) == null) {
          throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, "unknown operation reservation");
        }
      }
    }
    return execute(command);
  }
  static boolean isMutation(String operation) { return MUTATIONS.contains(operation); }
  synchronized Optional<Result> recoverOperation(DomainCommand command) {
    try {
      validate(command);
      Object result = map.recoverService(command.identity.orElseThrow(), request(command));
      return result instanceof Result original ? Optional.of(original) : Optional.empty();
    } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
  }
  public synchronized boolean cancelReserved(BoundedIMapApi.MutationIdentity identity) {
    boolean existed = map.servicePending(identity);
    if (existed) { map.abandonService(identity); cancellations++; }
    return existed;
  }

  public synchronized OperationStatus queryOperationOutcome(BoundedIMapApi.MutationIdentity identity) {
    Objects.requireNonNull(identity);
    Object result = map.serviceOutcome(identity);
    if (result instanceof Result retained) return new OperationStatus("completed", Optional.of(retained));
    if (result instanceof BoundedIMapApi.LegacyOutcome legacy) {
      Result converted;
      if (legacy.items().isEmpty()) {
        String kind = switch (legacy.outcome().status()) {
          case APPLIED -> "applied"; case NOT_APPLIED -> "not_applied"; case OUTCOME_UNKNOWN -> "outcome_unknown";
        };
        converted = new Result(kind, Optional.empty(), legacy.outcome().version(), OptionalLong.empty(),
            List.of(), 0, legacy.outcome().version().isPresent(), Optional.empty());
      } else {
        List<Item> items = legacy.items().stream().map(item -> new Item(item.index(), item.key(),
            item.replaced() ? "replaced" : "absent", Optional.empty(), item.version(), OptionalLong.empty())).toList();
        converted = items("bulk", items);
      }
      return new OperationStatus("completed", Optional.of(converted));
    }
    return new OperationStatus(map.servicePending(identity) ? "pending" : "unknown", Optional.empty());
  }

  public List<SnapshotEntry> snapshot(long time) {
    List<SnapshotEntry> result;
    synchronized (this) {
      advance(time);
      result = map.retainedEntries().stream().map(entry -> new SnapshotEntry(entry.key(), entry.value(),
          entry.version(), ttl(entry.expiresAt()))).toList();
    }
    wakeListeners();
    return result;
  }

  public Statistics stats(long time) {
    Statistics result;
    synchronized (this) {
      advance(time);
      result = new Statistics(map.serviceLiveCount(), map.serviceValueBytes(),
        map.serviceRetainedCount(), listeners.values().stream().filter(state -> !state.terminal).count(),
        listeners.values().stream().mapToLong(state -> state.queue.size()).sum(),
        map.servicePendingCount(), rejections, cancellations, requests, activeStreams, activeCursors);
    }
    wakeListeners();
    return result;
  }

  public synchronized ListenerHandle registerListener(long id, ListenerOptions options) {
    if (map.isDestroyed()) throw domain(IMapDomainException.Kind.MAP_DESTROYED, "map is destroyed");
    if (id <= 0 || listeners.containsKey(id)) invalid("invalid listener registration");
    if (listeners.size() >= bounds.maxEntries() || options.queueCapacity() > bounds.maxPageItems()
        || options.maxValueBytes() > bounds.maxResponseBytes()) {
      throw domain(IMapDomainException.Kind.CAPACITY, "listener capacity");
    }
    listeners.put(id, new ListenerState(options));
    return new ListenerHandle(id);
  }

  public synchronized List<Event> drainListener(long id) {
    ListenerState state = listeners.get(id);
    if (state == null) return List.of();
    List<Event> events = List.copyOf(state.queue);
    state.queue.clear();
    if (state.terminal) listeners.remove(id);
    return events;
  }

  public boolean removeListener(long id) {
    ListenerState removed;
    synchronized (this) {
      removed = listeners.remove(id);
      if (removed != null) pendingWake.add(removed.wake);
    }
    wakeListeners();
    return removed != null;
  }
  public synchronized void repairListener(long id) {
    ListenerState state = listeners.get(id);
    if (state == null) throw domain(IMapDomainException.Kind.UNKNOWN_LISTENER, "unknown listener");
    state.queue.clear(); state.gap = false;
  }

  public final class ListenerHandle implements AutoCloseable {
    private final long id;
    private boolean closed;
    private ListenerHandle(long id) { this.id = id; }
    public long id() { return id; }
    public List<Event> drain() { synchronized (UnifiedIMapService.this) {
      return closed ? List.of() : drainListener(id);
    } }
    @Override public void close() {
      boolean remove;
      synchronized (UnifiedIMapService.this) { remove = !closed; closed = true; }
      if (remove) removeListener(id);
    }
  }

  public IMapFlowPublisher<BoundedIMapApi.ScanItem> pages(BoundedIMapApi.Projection projection,
      int pageItems, LongSupplier clock) {
    synchronized (this) { advance(clock.getAsLong()); }
    wakeListeners();
    BoundedIMapApi.CloseableIterator<BoundedIMapApi.ScanItem> iterator = map.iterator(projection, pageItems, clock);
    try { synchronized (this) { reserveStream(); activeCursors++; } }
    catch (RuntimeException failure) { iterator.close(); throw failure; }
    return new IMapFlowPublisher<>(new IMapFlowPublisher.Source<>() {
      @Override public IMapFlowPublisher.Pull<BoundedIMapApi.ScanItem> poll() {
        try { synchronized (UnifiedIMapService.this) {
            advance(clock.getAsLong());
            return iterator.hasNext() ? IMapFlowPublisher.Pull.item(iterator.next())
                : IMapFlowPublisher.Pull.end();
        } } catch (RuntimeException failure) { throw IMapDomainException.translate(failure); }
        finally { wakeListeners(); }
      }
      @Override public void close() { iterator.close(); synchronized (UnifiedIMapService.this) {
        activeStreams--; activeCursors--;
      } }
    }, pageItems);
  }

  public IMapFlowPublisher<Event> events(ListenerHandle handle) {
    synchronized (this) {
      ListenerState state = listeners.get(handle.id());
      if (state == null || state.streamAttached) invalid("listener handle is closed or already attached");
      reserveStream(); state.streamAttached = true;
    }
    var publisher = new IMapFlowPublisher<Event>(new IMapFlowPublisher.Source<>() {
      final ArrayDeque<Event> pending = new ArrayDeque<>();
      boolean terminalBatch;
      @Override public IMapFlowPublisher.Pull<Event> poll() {
        synchronized (UnifiedIMapService.this) {
          if (handle.closed || (!terminalBatch && !listeners.containsKey(handle.id()))) {
            pending.clear();
            throw domain(IMapDomainException.Kind.UNKNOWN_LISTENER, "listener removed");
          }
          if (pending.isEmpty()) {
            ListenerState state = listeners.get(handle.id());
            terminalBatch = terminalBatch || (state != null && state.terminal);
            pending.addAll(handle.drain());
          }
          if (pending.isEmpty()) {
            if (terminalBatch) return IMapFlowPublisher.Pull.end();
            return IMapFlowPublisher.Pull.pending();
          }
          return IMapFlowPublisher.Pull.item(pending.removeFirst());
        }
      }
      @Override public void close() { pending.clear(); handle.close(); synchronized (UnifiedIMapService.this) { activeStreams--; } }
    }, bounds.maxPageItems());
    synchronized (this) {
      ListenerState state = listeners.get(handle.id());
      if (state != null) state.wake = publisher::signal;
    }
    return publisher;
  }

  private Result read(DomainCommand c) {
    if (!Set.of("query_operation_outcome", "local_statistics", "capability_descriptor", "advance")
        .contains(c.operation) && map.isDestroyed()) {
      throw domain(IMapDomainException.Kind.MAP_DESTROYED, "map is destroyed");
    }
    Optional<BoundedIMapApi.EntryView> current = pointRead(c.operation) ? map.getEntryView(c.key, now)
        : Optional.empty();
    return switch (c.operation) {
      case "get", "get_entry_view" -> current.map(view -> point(c.operation.equals("get")
          ? "present" : "entry_view", Optional.of(view.value()), view)).orElse(simple("absent"));
      case "contains_key" -> current.map(view -> flagged(point("present", Optional.empty(), view), true))
          .orElse(simple("absent"));
      case "contains_entry" -> current.map(view -> flagged(point(view.value().equals(required(c.expected))
          ? "present" : "mismatch", Optional.empty(), view), view.value().equals(required(c.expected)))).orElse(simple("absent"));
      case "get_or_default" -> current.map(view -> point("present", Optional.of(view.value()), view))
          .orElse(new Result("absent", Optional.of(required(c.value)), OptionalLong.empty(),
              OptionalLong.empty(), List.of(), 0, false, Optional.empty()));
      case "remaining_ttl" -> current.map(view -> point("present", Optional.empty(), view))
          .orElse(simple("absent"));
      case "size" -> numbered("counted", map.size(now), map.isEmpty(now));
      case "is_empty" -> numbered(map.isEmpty(now) ? "empty" : "non_empty", map.size(now), map.isEmpty(now));
      case "contains_value_bounded" -> {
        BoundedIMapApi.MatchResult match = map.containsValue(required(c.value), c.cursor, c.limit, now);
        yield new Result(switch (match.match()) { case PRESENT -> "present"; case ABSENT -> "absent"; case INCOMPLETE -> "partial"; }, Optional.empty(), OptionalLong.empty(),
            OptionalLong.empty(), List.of(), 0, match.match() == BoundedIMapApi.Match.PRESENT,
            Optional.empty(), match.nextCursor());
      }
      case "key_set_page", "values_page", "entry_set_page" -> {
        BoundedIMapApi.Projection projection = c.operation.equals("key_set_page")
            ? BoundedIMapApi.Projection.KEYS : c.operation.equals("values_page")
            ? BoundedIMapApi.Projection.VALUES : BoundedIMapApi.Projection.ENTRIES;
        var page = map.scan(projection, c.cursor, c.limit, now);
        var items = new ArrayList<Item>();
        for (int index = 0; index < page.items().size(); index++) {
          var item = page.items().get(index);
          items.add(new Item(index, item.key().orElse(EMPTY), "present", item.value(),
              OptionalLong.empty(), OptionalLong.empty()));
        }
        yield new Result(page.complete() ? "page" : "partial", Optional.empty(), OptionalLong.empty(),
            OptionalLong.empty(), items, items.size(), page.complete(), Optional.empty(), page.nextCursor());
      }
      case "get_all" -> {
        List<Item> items = new ArrayList<>();
        for (int index = 0; index < c.keys.size(); index++) {
          BytesValue key = c.keys.get(index);
          Optional<BoundedIMapApi.EntryView> value = map.getEntryView(key, now);
          items.add(item(index, key, value.isPresent() ? "present" : "absent", value));
        }
        yield items("bulk", items);
      }
      case "entry_listener" -> {
        registerListener(c.listenerId, new ListenerOptions(c.includeValue, bounds.maxValueBytes(),
            c.limit, Optional.empty(), Set.of()));
        yield numbered("listener_added", c.listenerId, true);
      }
      case "remove_entry_listener" -> {
        boolean removed = removeListener(c.listenerId);
        yield numbered(removed ? "listener_removed" : "absent", 0, removed);
      }
      case "capability_descriptor" -> numbered("capabilities", IMapCapabilityDescriptor.local(bounds).plannedOperations().size(), false);
      case "query_operation_outcome" -> {
        OperationStatus status = queryOperationOutcome(c.identity.orElseThrow(() -> domain(
            IMapDomainException.Kind.INVALID_ARGUMENT, "identity required")));
        yield status.result().map(result -> new Result(status.kind(), result.value(), result.version(),
            result.remainingTtl(), result.items(), result.number(), result.flag(), result.token()))
            .orElse(simple(status.kind()));
      }
      case "local_statistics" -> numbered("statistics", stats(now).liveEntries(), map.isDestroyed());
      case "advance" -> simple("advanced");
      default -> throw domain(IMapDomainException.Kind.UNSUPPORTED_OPERATION, "unsupported operation");
    };
  }

  private Result mutate(DomainCommand c) {
    Optional<BoundedIMapApi.EntryView> previous = map.getEntryView(c.key, now);
    String operation = c.operation;
    if (operation.equals("put_all_detailed") || operation.equals("replace_all_detailed")) {
      List<Item> result = new ArrayList<>();
      for (int i = 0; i < c.entries.size(); i++) {
        var entry = c.entries.get(i);
        Optional<BoundedIMapApi.EntryView> old = map.getEntryView(entry.key(), now);
        if (operation.equals("replace_all_detailed") && old.isEmpty()) {
          result.add(item(i, entry.key(), "absent", Optional.empty()));
        } else {
          Long expiry = operation.equals("replace_all_detailed") ? expiry(old)
              : expiry(c.ttl, old);
          map.put(entry.key(), entry.value(), expiry, now);
          emit(old.isPresent() ? "updated" : "added", entry.key(),
              old.map(BoundedIMapApi.EntryView::value), Optional.of(entry.value()), map.revision());
          var changed = map.getEntryView(entry.key(), now).orElseThrow();
          result.add(new Item(i, entry.key(), old.isPresent() ? "replaced" : "inserted", Optional.empty(),
              OptionalLong.of(changed.version()), changed.remainingTtl()));
        }
      }
      return items("bulk", result);
    }
    if (operation.equals("remove_all_detailed")) {
      List<Item> result = new ArrayList<>();
      for (int i = 0; i < c.keys.size(); i++) {
        var old = map.getEntryView(c.keys.get(i), now);
        map.evict(c.keys.get(i), now);
        if (old.isPresent()) emit("removed", c.keys.get(i), old.map(BoundedIMapApi.EntryView::value),
            Optional.empty(), map.revision());
        result.add(new Item(i, c.keys.get(i), old.isPresent() ? "removed" : "absent",
            old.map(BoundedIMapApi.EntryView::value), OptionalLong.empty(), OptionalLong.empty()));
      }
      return items("bulk", result);
    }
    if (Set.of("clear_detailed", "evict_all_detailed", "destroy").contains(operation)) {
      BoundedIMapApi.RemovalReceipt receipt = operation.equals("destroy") ? map.destroy(now)
          : operation.equals("clear_detailed") ? map.clear(now) : map.evictAll(now);
      return new Result(operation.equals("destroy") ? "destroyed" : operation.equals("clear_detailed")
          ? "cleared" : "evicted", Optional.empty(), OptionalLong.empty(),
          OptionalLong.empty(), List.of(), receipt.removed(), false, Optional.empty());
    }
    if (Set.of("replace_if_version", "remove_if_version", "set_ttl_if_version").contains(operation)) {
      if (previous.isEmpty()) return simple("absent");
      if (!token(previous.orElseThrow()).equals(c.token.orElseThrow(() -> domain(
          IMapDomainException.Kind.INVALID_ARGUMENT, "version token required")))) {
        return point("mismatch", Optional.empty(), previous.orElseThrow());
      }
    }
    if (operation.equals("put_if_absent") && previous.isPresent()) {
      return point("present", Optional.of(previous.orElseThrow().value()), previous.orElseThrow());
    }
    if (Set.of("replace_if_present", "conditional_replace", "conditional_remove", "set_ttl")
        .contains(operation) && previous.isEmpty()) return simple("absent");
    if (Set.of("conditional_replace", "conditional_remove").contains(operation)
        && !previous.orElseThrow().value().equals(required(c.expected))) {
      return point("mismatch", Optional.of(previous.orElseThrow().value()), previous.orElseThrow());
    }
    if (Set.of("delete", "get_and_remove", "conditional_remove", "evict", "try_remove",
        "remove_if_version").contains(operation)) {
      boolean changed = map.evict(c.key, now);
      if (operation.equals("try_remove")) return flagged(versioned("applied",
          changed ? OptionalLong.of(map.revision()) : OptionalLong.empty()), changed);
      if (!changed) return simple("absent");
      return new Result(operation.equals("evict") ? "evicted" : "removed",
          operation.equals("get_and_remove") || operation.equals("conditional_remove")
              ? previous.map(BoundedIMapApi.EntryView::value) : Optional.empty(),
          OptionalLong.of(map.revision()), OptionalLong.empty(), List.of(), 0, true, Optional.empty());
    }
    if (Set.of("set_ttl", "set_ttl_if_version").contains(operation)) {
      map.put(c.key, previous.orElseThrow().value(), expiry(c.ttl, previous), now);
      return appliedPoint("replaced", Optional.empty(), map.getEntryView(c.key, now).orElseThrow());
    }
    BytesValue value = required(c.value);
    map.put(c.key, value, expiry(c.ttl, previous), now);
    String kind = Set.of("set", "try_put").contains(operation) ? "applied"
        : previous.isPresent() ? "replaced" : "inserted";
    Optional<BytesValue> returned = Set.of("get_and_put", "conditional_replace", "replace_if_present",
        "replace_if_version").contains(operation) ? previous.map(BoundedIMapApi.EntryView::value) : Optional.empty();
    return appliedPoint(kind, returned, map.getEntryView(c.key, now).orElseThrow());
  }

  private void validate(DomainCommand c) {
    Objects.requireNonNull(c.key); Objects.requireNonNull(c.ttl);
    if (c.key.size() > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.KEY_TOO_LARGE, "key bound");
    for (var value : List.of(c.value, c.expected)) {
      if (value.isPresent() && value.orElseThrow().size() > bounds.maxValueBytes()) {
        throw domain(IMapDomainException.Kind.VALUE_TOO_LARGE, "value bound");
      }
    }
    if (c.value.isPresent() && c.value.orElseThrow().size() > bounds.maxResponseBytes()) {
      throw domain(IMapDomainException.Kind.RESPONSE_TOO_LARGE, "response value bound");
    }
    if (c.limit <= 0) invalid("limit must be positive");
    if (c.keys.size() > 256 || c.entries.size() > 256) throw domain(IMapDomainException.Kind.REQUEST_TOO_LARGE, "request count");
    long requestBytes = 0, responseBytes = 0;
    var seen = new HashSet<BytesValue>();
    var existing = new HashMap<BytesValue, BoundedIMapApi.RetainedEntry>();
    map.retainedEntries().stream().filter(entry -> entry.expiresAt() == null || entry.expiresAt() > c.now)
        .forEach(entry -> existing.put(entry.key(), entry));
    int inserted = 0;
    for (var entry : c.entries) {
      if (!seen.add(entry.key())) invalid("duplicate key");
      if (entry.key().size() > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.KEY_TOO_LARGE, "key bound");
      if (entry.value().size() > bounds.maxValueBytes()) throw domain(IMapDomainException.Kind.VALUE_TOO_LARGE, "value bound");
      requestBytes += 8L + entry.key().size() + entry.value().size();
      responseBytes += 17L + entry.key().size();
      if (entry.value().size() > bounds.maxResponseBytes()) throw domain(
          IMapDomainException.Kind.RESPONSE_TOO_LARGE, "response value bound");
      if (c.operation.equals("put_all_detailed")) {
        if (!existing.containsKey(entry.key())) inserted++;
        if (c.ttl.kind() == TtlDirective.Kind.PRESERVE && !existing.containsKey(entry.key())) {
          throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, "preserve requires existing entry");
        }
      }
    }
    for (var key : c.keys) {
      if (key.size() > bounds.maxKeyBytes()) throw domain(IMapDomainException.Kind.KEY_TOO_LARGE, "key bound");
      requestBytes += 4L + key.size(); responseBytes += 17L + key.size();
      if (c.operation.equals("get_all") || c.operation.equals("remove_all_detailed")) {
        var old = existing.get(key);
        if (old != null) responseBytes += old.value().size();
      }
    }
    if (requestBytes > BoundedIMapApi.MAX_REQUEST_BYTES) throw domain(IMapDomainException.Kind.REQUEST_TOO_LARGE, "request bytes");
    if (responseBytes > bounds.maxResponseBytes()) throw domain(IMapDomainException.Kind.RESPONSE_TOO_LARGE, "response bytes");
    if (existing.size() + inserted > bounds.maxEntries()) throw domain(IMapDomainException.Kind.CAPACITY, "entry capacity");
    if (c.ttl.kind() == TtlDirective.Kind.EXPIRE_AFTER) {
      try { Math.addExact(c.now, c.ttl.ticks()); }
      catch (ArithmeticException failure) { invalid("expiry overflow"); }
    }
    if (Set.of("get_all", "remove_all_detailed").contains(c.operation) && c.keys.isEmpty()) invalid("empty keys");
    if (Set.of("put_all_detailed", "replace_all_detailed").contains(c.operation) && c.entries.isEmpty()) invalid("empty entries");
    if (Set.of("put", "set", "get_and_put", "try_put", "put_if_absent", "replace_if_present",
        "conditional_replace", "replace_if_version", "get_or_default", "contains_value_bounded")
        .contains(c.operation)) required(c.value);
    if (Set.of("conditional_replace", "conditional_remove", "contains_entry").contains(c.operation)) required(c.expected);
  }

  private void advance(long time) {
    if (time < now || time < 0) invalid("logical time regressed");
    List<BoundedIMapApi.RetainedEntry> before = map.retainedEntries();
    now = time;
    map.expireForService(now);
    for (var entry : before) if (entry.expiresAt() != null && entry.expiresAt() <= now) {
      emit("expired", entry.key(), Optional.of(entry.value()), Optional.empty(), map.revision());
    }
  }

  private void publishChanges(String operation, List<BoundedIMapApi.RetainedEntry> before,
      List<BoundedIMapApi.RetainedEntry> after) {
    if (operation.equals("clear_detailed") || operation.equals("destroy")) {
      if (operation.equals("destroy") || !before.isEmpty()) {
        emit(operation.equals("destroy") ? "destroyed" : "cleared", EMPTY, Optional.empty(), Optional.empty(), map.revision());
      }
      if (operation.equals("destroy")) listeners.values().forEach(state -> state.terminal = true);
      return;
    }
    var old = new TreeMap<BytesValue, BoundedIMapApi.RetainedEntry>();
    var current = new TreeMap<BytesValue, BoundedIMapApi.RetainedEntry>();
    before.forEach(value -> old.put(value.key(), value));
    after.forEach(value -> current.put(value.key(), value));
    var keys = new java.util.TreeSet<BytesValue>(); keys.addAll(old.keySet()); keys.addAll(current.keySet());
    for (var key : keys) {
      var previous = old.get(key); var value = current.get(key);
      if (value == null) emit(operation.equals("evict") || operation.equals("evict_all_detailed")
          ? "evicted" : "removed", key, Optional.of(previous.value()), Optional.empty(), map.revision());
      else if (previous == null || previous.version() != value.version()) {
        emit(previous == null ? "added" : "updated", key,
            previous == null ? Optional.empty() : Optional.of(previous.value()), Optional.of(value.value()), value.version());
      }
    }
  }

  private void emit(String kind, BytesValue key, Optional<BytesValue> oldValue,
      Optional<BytesValue> value, long version) {
    watermark++;
    for (var state : listeners.values()) {
      if (state.gap || state.terminal || (state.options.exactKey().isPresent()
          && !state.options.exactKey().orElseThrow().equals(key))
          || (!state.options.kinds().isEmpty() && !state.options.kinds().contains(kind))) continue;
      int bytes = oldValue.map(BytesValue::size).orElse(0) + value.map(BytesValue::size).orElse(0);
      if (state.queue.size() >= state.options.queueCapacity()
          || (state.options.includeValue() && bytes > state.options.maxValueBytes())) {
        state.queue.clear(); state.gap = true;
        state.queue.add(new Event("gap", EMPTY, Optional.empty(), Optional.empty(), version, watermark));
      } else state.queue.add(new Event(kind, key, state.options.includeValue() ? oldValue : Optional.empty(),
          state.options.includeValue() ? value : Optional.empty(), version, watermark));
    }
    listeners.values().stream().map(state -> state.wake).forEach(pendingWake::add);
  }

  private void wakeListeners() {
    if (Thread.holdsLock(this)) return;
    List<Runnable> wake;
    synchronized (this) { wake = List.copyOf(pendingWake); pendingWake.clear(); }
    wake.forEach(Runnable::run);
  }

  private Request request(DomainCommand c) {
    return new Request(c.operation, c.key, c.value, c.expected, List.copyOf(c.keys),
        List.copyOf(c.entries), c.ttl, c.token);
  }
  private static boolean pointRead(String op) {
    return Set.of("get", "get_entry_view", "contains_key", "contains_entry", "get_or_default",
        "remaining_ttl").contains(op);
  }
  private Long expiry(TtlDirective directive, Optional<BoundedIMapApi.EntryView> previous) {
    return switch (directive.kind()) {
      case ETERNAL -> null;
      case EXPIRE_AFTER -> Math.addExact(now, directive.ticks());
      case PRESERVE -> { if (previous.isEmpty()) invalid("preserve requires existing entry"); yield expiry(previous); }
    };
  }
  private Long expiry(Optional<BoundedIMapApi.EntryView> previous) {
    return previous.orElseThrow().remainingTtl().isPresent()
        ? Math.addExact(now, previous.orElseThrow().remainingTtl().getAsLong()) : null;
  }
  private OptionalLong ttl(Long expiry) { return expiry == null ? OptionalLong.empty() : OptionalLong.of(expiry - now); }
  private VersionToken token(BoundedIMapApi.EntryView view) { return new VersionToken(generation, mapId, view.version()); }
  private Result point(String kind, Optional<BytesValue> value, BoundedIMapApi.EntryView view) {
    return new Result(kind, value, OptionalLong.of(view.version()), view.remainingTtl(), List.of(), 0,
        false, Optional.of(token(view)));
  }
  private static Result simple(String kind) { return new Result(kind, Optional.empty(), OptionalLong.empty(),
      OptionalLong.empty(), List.of(), 0, false, Optional.empty()); }
  private static Result versioned(String kind, OptionalLong version) { return new Result(kind, Optional.empty(),
      version, OptionalLong.empty(), List.of(), 0, false, Optional.empty()); }
  private static Result numbered(String kind, long number, boolean flag) { return new Result(kind,
      Optional.empty(), OptionalLong.empty(), OptionalLong.empty(), List.of(), number, flag, Optional.empty()); }
  private static Result items(String kind, List<Item> items) { return new Result(kind, Optional.empty(),
      OptionalLong.empty(), OptionalLong.empty(), items, items.size(), false, Optional.empty()); }
  private Result appliedPoint(String kind, Optional<BytesValue> value, BoundedIMapApi.EntryView view) {
    Result result = point(kind, value, view);
    return new Result(result.kind(), result.value(), result.version(), result.remainingTtl(),
        result.items(), result.number(), true, result.token());
  }
  private static Result flagged(Result result, boolean flag) {
    return new Result(result.kind(), result.value(), result.version(), result.remainingTtl(), result.items(),
        result.number(), flag, result.token(), result.cursor());
  }
  private void reserveStream() {
    if (activeStreams >= bounds.maxEntries()) throw domain(IMapDomainException.Kind.CAPACITY, "stream capacity");
    activeStreams++;
  }
  private static Item item(int index, BytesValue key, String kind, Optional<BoundedIMapApi.EntryView> view) {
    return new Item(index, key, kind, view.map(BoundedIMapApi.EntryView::value), view.isPresent()
        ? OptionalLong.of(view.orElseThrow().version()) : OptionalLong.empty(), view.isPresent()
        ? view.orElseThrow().remainingTtl() : OptionalLong.empty());
  }
  private static BytesValue required(Optional<BytesValue> value) {
    return value.orElseThrow(() -> domain(IMapDomainException.Kind.INVALID_ARGUMENT, "required value missing"));
  }
  private static IMapDomainException domain(IMapDomainException.Kind kind, String detail) {
    return new IMapDomainException(kind, detail);
  }
  private static void invalid(String detail) { throw domain(IMapDomainException.Kind.INVALID_ARGUMENT, detail); }
}
