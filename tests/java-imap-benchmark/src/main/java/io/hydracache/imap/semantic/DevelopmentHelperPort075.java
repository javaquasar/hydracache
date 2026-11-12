package io.hydracache.imap.semantic;

import java.util.List;
import java.util.Objects;
import java.util.Set;
import java.util.concurrent.Executor;
import java.util.function.LongSupplier;

/** Local helper adapter consistency and immutable development grants; not member authentication. */
public final class DevelopmentHelperPort075 {
  public enum Right { READ, WRITE, LISTEN, ADMIN }
  public record Grant(long tenant, long namespace, long generation, long epoch, Set<Right> rights,
      boolean allKeys, List<BytesValue> prefixes) {
    public Grant {
      rights = Set.copyOf(rights); prefixes = List.copyOf(prefixes);
      if (tenant <= 0 || namespace <= 0 || generation <= 0 || epoch <= 0 || prefixes.size() > 16
          || allKeys && !prefixes.isEmpty() || !allKeys && prefixes.isEmpty()
          || prefixes.stream().anyMatch(p -> p.size() == 0 || p.size() > 4096)
          || prefixes.stream().mapToInt(BytesValue::size).sum() > 16384) throw new IllegalArgumentException("development grant bound");
    }
    boolean permits(BytesValue key) {
      if (allKeys) return true;
      byte[] bytes = key.copy();
      for (var prefix : prefixes) {
        byte[] part = prefix.copy();
        if (bytes.length >= part.length && java.util.Arrays.equals(bytes, 0, part.length, part, 0, part.length)) return true;
      }
      return false;
    }
  }
  public static final class Prepared<T> {
    private final DevelopmentHelperPort075 owner;
    private final DevelopmentBackendPort075.Binding binding;
    private final Grant grant;
    private final BoundedIMapApi.MutationIdentity invocation;
    private final IMapLocalHelper.Command<T> command;
    private Prepared(DevelopmentHelperPort075 owner, DevelopmentBackendPort075.Binding binding, Grant grant,
        BoundedIMapApi.MutationIdentity invocation, IMapLocalHelper.Command<T> command) {
      this.owner = owner; this.binding = binding; this.grant = grant; this.invocation = invocation; this.command = command;
    }
    public long deadline() { return command.deadline(); }
  }
  private final DevelopmentBackendPort075.Binding binding;
  private final Grant grant;
  private final UnifiedIMapService service;
  private final UnifiedIMapAsyncFacade facade;
  private final LongSupplier clock;
  public DevelopmentHelperPort075(DevelopmentBackendPort075.Binding binding, Grant grant, UnifiedIMapService service,
      Executor executor, LongSupplier clock, int maxPending, DevelopmentBackendPort075.Mode mode) {
    new DevelopmentBackendPort075(binding, service, mode);
    this.binding = binding; this.grant = Objects.requireNonNull(grant); this.service = service;
    if (grant.tenant() != binding.tenant() || grant.namespace() != binding.namespace()
        || grant.generation() != binding.generation()) throw new SecurityException("helper grant scope mismatch");
    this.clock = Objects.requireNonNull(clock); facade = new UnifiedIMapAsyncFacade(service, executor, clock, maxPending);
  }
  private void validate(DevelopmentBackendPort075.Binding context, BoundedIMapApi.MutationIdentity identity, IMapLocalHelper.Command<?> command) {
    if (!binding.equals(context)) throw new DevelopmentBackendPort075.BoundaryException(DevelopmentBackendPort075.Failure.CONTEXT_MISMATCH);
    if (!Objects.requireNonNull(identity).client().equals(binding.client())
        || IMapLocalHelper.identity(command).filter(id -> !id.equals(identity)).isPresent())
      throw new DevelopmentBackendPort075.BoundaryException(DevelopmentBackendPort075.Failure.IDENTITY_MISMATCH);
    service.prepareHelperRequest(command, clock.getAsLong());
    Right right; boolean wide;
    if (command instanceof IMapLocalHelper.EntryViews) { right = Right.READ; wide = false; }
    else if (command instanceof IMapLocalHelper.VersionBulk) { right = Right.WRITE; wide = false; }
    else if (command instanceof IMapLocalHelper.QueryOutcomes) { right = Right.READ; wide = true; }
    else if (command instanceof IMapLocalHelper.ListenerSnapshot) { right = Right.LISTEN; wide = true; }
    else { right = Right.ADMIN; wide = true; }
    if (!grant.rights().contains(right) || wide && !grant.allKeys()) throw new SecurityException("helper permission denied");
    if (command instanceof IMapLocalHelper.EntryViews request && request.keys().stream().anyMatch(k -> !grant.permits(k))) throw new SecurityException("helper key denied");
    if (command instanceof IMapLocalHelper.VersionBulk request && request.entries().stream().anyMatch(e -> !grant.permits(e.key()))) throw new SecurityException("helper key denied");
  }
  public <T> Prepared<T> prepare(DevelopmentBackendPort075.Binding context, BoundedIMapApi.MutationIdentity identity, IMapLocalHelper.Command<T> command) {
    validate(context, identity, Objects.requireNonNull(command)); return new Prepared<>(this, binding, grant, identity, command);
  }
  public <T> UnifiedIMapAsyncFacade.HelperInvocation<T> invoke(Prepared<T> prepared) {
    Objects.requireNonNull(prepared);
    if (prepared.owner != this || !grant.equals(prepared.grant)) throw new SecurityException("helper owner or grant changed");
    validate(prepared.binding, prepared.invocation, prepared.command);
    return facade.invokeHelper(prepared.command);
  }
}
