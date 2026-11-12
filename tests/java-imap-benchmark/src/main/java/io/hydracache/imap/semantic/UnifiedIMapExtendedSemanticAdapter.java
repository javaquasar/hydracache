package io.hydracache.imap.semantic;

import static io.hydracache.imap.semantic.ExtendedSemanticScenario.*;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;
import java.util.TreeMap;

/** Actual-owner v2 bridge. Only token references and real resource handles are retained here. */
public final class UnifiedIMapExtendedSemanticAdapter implements ExtendedMapSemanticAdapter {
  enum Fault { NONE, DROP_EVENT, WRONG_TTL, REVERSE_BULK, REUSE_TOKEN, WRONG_OWNERS, WRONG_CLOCK, WRONG_CUT }
  private UnifiedIMapService service;
  private UnifiedIMapService.ListenerHandle listener;
  private UnifiedIMapService.SnapshotLease held;
  private final AdapterIdentity identity;
  private final Fault fault;
  private final Map<String, Optional<UnifiedIMapService.VersionToken>> tokens = new TreeMap<>();
  private long tick;
  private boolean closed;

  public UnifiedIMapExtendedSemanticAdapter(String name, UnifiedIMapService service) { this(name, service, Fault.NONE); }
  private UnifiedIMapExtendedSemanticAdapter(String name, UnifiedIMapService service, Fault fault) {
    text(name); this.service = Objects.requireNonNull(service); this.fault = Objects.requireNonNull(fault);
    identity = new AdapterIdentity("local-reference", name, "extended-semantic-v2-not-production");
    register();
  }
  public static UnifiedIMapExtendedSemanticAdapter fresh(String name) { return fresh(name, Fault.NONE); }
  static UnifiedIMapExtendedSemanticAdapter fresh(String name, Fault fault) {
    return new UnifiedIMapExtendedSemanticAdapter(name, new UnifiedIMapService(
        new BoundedIMapApi.Bounds(1024, 4096, 256, 64, 256, 1024 * 1024, 64), 1), fault);
  }
  private void register() {
    listener = service.registerListener(Long.MAX_VALUE, new UnifiedIMapService.ListenerOptions(true, 8192, MAX_ITEMS, Optional.empty(), Set.of()));
  }
  @Override public AdapterIdentity identity() { return identity; }
  @Override public Set<Op> extendedCapabilities() { return Set.of(Op.values()); }
  @Override public synchronized Observation executeExtended(Step step) {
    if (closed) throw new IllegalStateException("extended adapter closed");
    Objects.requireNonNull(step);
    Result result;
    try { result = execute(step); }
    catch (IMapDomainException failure) { result = Result.simple("error:" + failure.kind()); }
    var events = listener.drain().stream().map(event -> new Event(event.kind(), event.key(), event.oldValue(), event.value(), event.watermark())).toList();
    var snapshot = service.diagnosticSnapshot(tick).stream().map(entry -> new Row(entry.key(), entry.value(),
        entry.remainingTtl().isPresent() ? TtlState.expiring(entry.remainingTtl().getAsLong()) : TtlState.eternal())).toList();
    if (fault == Fault.DROP_EVENT && !events.isEmpty()) events = events.subList(1, events.size());
    if (fault == Fault.WRONG_TTL && !snapshot.isEmpty()) {
      var wrong = new ArrayList<>(snapshot); Row row = wrong.get(0);
      wrong.set(0, new Row(row.key(), row.value(), row.ttl().kind() == TtlState.Kind.ETERNAL ? TtlState.expiring(1) : TtlState.eternal())); snapshot = wrong;
    }
    var statistics = service.diagnosticStats(tick);
    return new Observation(result, snapshot, events, relations(tokens), new Owners(fault == Fault.WRONG_OWNERS ? statistics.retainedOutcomes() + 1 : statistics.retainedOutcomes(),
         statistics.inflight(), statistics.listeners(), held != null && !held.entries().isEmpty()), fault == Fault.WRONG_CLOCK ? tick == Long.MAX_VALUE ? tick - 1 : tick + 1 : tick);
  }
  private Result execute(Step step) {
    return switch (step.operation()) {
      case CAPTURE_VERSION -> capture(step);
      case PUT, REPLACE_IF_VERSION, REMOVE_IF_VERSION, SET_TTL_IF_VERSION, EVICT, CLEAR, DESTROY, DISPATCH_PUT -> {
        var command = command(step);
        yield result(step.operation() == Op.DISPATCH_PUT ? service.executeReserved(command) : service.execute(command));
      }
      case RESERVE_PUT -> { service.beginOperation(command(step)); yield Result.simple("pending"); }
      case CANCEL_RESERVED -> Result.simple(service.cancelReserved(id(step.requests().get(0))) ? "cancelled" : "absent");
      case BULK_REPLACE, BULK_REMOVE, BULK_SET_TTL -> {
        var action = switch (step.operation()) { case BULK_REPLACE -> UnifiedIMapService.VersionAction.REPLACE; case BULK_REMOVE -> UnifiedIMapService.VersionAction.REMOVE; default -> UnifiedIMapService.VersionAction.SET_TTL; };
        var entries = step.entries().stream().map(entry -> new UnifiedIMapService.VersionInput(entry.key(), token(entry), entry.value(), entry.ttl())).toList();
        Result actual = result(service.versionBulk(action, entries, id(step.requests().get(0)), tick, Long.MAX_VALUE, step.loseResponse()));
        if (fault == Fault.REVERSE_BULK && actual.items().size() > 1) {
          var reversed = new ArrayList<>(actual.items()); Collections.reverse(reversed);
          actual = new Result(actual.kind(), actual.value(), actual.ttl(), reversed, actual.statuses(), actual.number());
        }
        yield actual;
      }
      case QUERY_OUTCOMES -> {
        var statuses = service.queryOutcomes(step.requests().stream().map(UnifiedIMapExtendedSemanticAdapter::id).toList(), 1024 * 1024);
        var results = statuses.stream().map(item -> new Status(step.requests().get(item.index()), item.status().kind(), item.status().result().map(UnifiedIMapExtendedSemanticAdapter::result))).toList();
        yield new Result("outcomes", Optional.empty(), TtlState.absent(), List.of(), results, results.size());
      }
      case RETIRE_OUTCOMES -> {
        Request request = step.requests().get(0);
        yield Result.numbered("retired", service.retireOutcomes(request.client(), 1, request.sequence()).removed());
      }
      case LISTENER_RESYNC, HOLD_LISTENER_SNAPSHOT -> {
        if (held != null) throw new IMapDomainException(IMapDomainException.Kind.CHECKPOINT_BUSY, "snapshot already held");
        var limits = step.limits().orElseThrow();
        var lease = service.snapshotListener(listener.id(), limits.maximumItems(), limits.maximumBytes(), tick);
        var entries = new ArrayList<Item>();
        for (var entry : lease.entries()) entries.add(new Item(entries.size(), entry.key(), "present", entry.value(), entry.remainingTtl().isPresent() ? TtlState.expiring(entry.remainingTtl().getAsLong()) : TtlState.eternal()));
        if (step.operation() == Op.HOLD_LISTENER_SNAPSHOT) held = lease; else lease.close();
        long watermark = lease.watermark();
        yield new Result("snapshot", Optional.empty(), TtlState.absent(), entries, List.of(), fault == Fault.WRONG_CUT ? watermark + 1 : watermark);
      }
      case CLOSE_LISTENER_SNAPSHOT -> { if (held != null) { held.close(); held = null; } yield Result.simple("closed"); }
      case CHECKPOINT_RESTORE -> {
        if (held != null || service.diagnosticStats(tick).inflight() != 0) throw new IMapDomainException(IMapDomainException.Kind.CHECKPOINT_BUSY, "close active owners before adapter restore");
        listener.close();
        try (var checkpoint = service.checkpoint()) { service = UnifiedIMapService.restore(checkpoint); }
        finally { register(); }
        yield Result.simple("restored");
      }
      case ADVANCE -> {
        long target;
        try { target = Math.addExact(tick, step.ticks()); }
        catch (ArithmeticException overflow) { throw new IMapDomainException(IMapDomainException.Kind.INVALID_ARGUMENT, "clock overflow"); }
        service.execute(new UnifiedIMapService.DomainCommand("advance").now(target)); tick = target; yield Result.simple("advanced");
      }
      case SIGNAL_GAP -> { service.signalListenerGap(listener.id()); yield Result.simple("gap"); }
      case RESOURCES -> Result.simple("resources");
    };
  }
  private Result capture(Step step) {
    Entry entry = step.entries().get(0); String label = entry.versionRef().orElseThrow();
    if (tokens.size() == MAX_ITEMS || tokens.containsKey(label)) throw new IllegalArgumentException("token reference bound/duplicate");
    var raw = service.execute(new UnifiedIMapService.DomainCommand("get_entry_view").key(entry.key()).now(tick));
    Optional<UnifiedIMapService.VersionToken> token = raw.token();
    if (fault == Fault.REUSE_TOKEN && token.isPresent()) token = tokens.values().stream().filter(Optional::isPresent).findFirst().orElse(token);
    tokens.put(label, token); return result(raw);
  }
  private UnifiedIMapService.DomainCommand command(Step step) {
    String operation = switch (step.operation()) {
      case PUT, RESERVE_PUT, DISPATCH_PUT -> "put";
      case REPLACE_IF_VERSION -> "replace_if_version"; case REMOVE_IF_VERSION -> "remove_if_version";
      case SET_TTL_IF_VERSION -> "set_ttl_if_version"; case EVICT -> "evict"; case CLEAR -> "clear_detailed"; case DESTROY -> "destroy";
      default -> throw new IllegalArgumentException("not a point mutation");
    };
    var command = new UnifiedIMapService.DomainCommand(operation).identity(id(step.requests().get(0))).now(tick).loseResponse(step.loseResponse());
    if (!step.entries().isEmpty()) {
      Entry entry = step.entries().get(0); command.key(entry.key()).ttl(entry.ttl()); entry.value().ifPresent(command::value);
      if (step.operation() == Op.REPLACE_IF_VERSION || step.operation() == Op.REMOVE_IF_VERSION || step.operation() == Op.SET_TTL_IF_VERSION) command.token(token(entry));
    }
    return command;
  }
  private UnifiedIMapService.VersionToken token(Entry entry) {
    var found = tokens.get(entry.versionRef().orElseThrow());
    if (found == null || found.isEmpty()) throw new IllegalArgumentException("unknown/absent token reference");
    return found.orElseThrow();
  }
  static BoundedIMapApi.MutationIdentity id(Request request) { return new BoundedIMapApi.MutationIdentity(request.client(), request.sequence()); }
  static Result result(UnifiedIMapService.Result result) {
    return new Result(result.kind(), result.value(), ttl(result.kind(), result.version().isPresent(), result.remainingTtl()),
        result.items().stream().map(item -> new Item(item.index(), item.key(), item.kind(), item.value(), ttl(item.kind(), item.version().isPresent(), item.remainingTtl()))).toList(), List.of(), result.number());
  }
  private static TtlState ttl(String kind, boolean version, java.util.OptionalLong remaining) {
    if (!version || kind.equals("removed") || kind.equals("evicted")) return TtlState.absent();
    return remaining.isPresent() ? TtlState.expiring(remaining.getAsLong()) : TtlState.eternal();
  }
  static <T> Map<String, Boolean> relations(Map<String, Optional<T>> tokens) {
    var result = new TreeMap<String, Boolean>();
    tokens.forEach((left, a) -> tokens.forEach((right, b) -> result.put(left + "=" + right, a.isPresent() && a.equals(b))));
    return result;
  }
  @Override public synchronized void close() { if (!closed) { if (held != null) held.close(); listener.close(); tokens.clear(); closed = true; } }
}
