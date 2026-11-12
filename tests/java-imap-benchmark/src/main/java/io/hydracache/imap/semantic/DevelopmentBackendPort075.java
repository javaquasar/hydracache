package io.hydracache.imap.semantic;

import java.nio.charset.StandardCharsets;
import java.util.Objects;

/** Test-only transport seam. A binding checks adapter consistency, not member authentication. */
public final class DevelopmentBackendPort075 {
  public enum Operation {
    GET, PUT, DELETE, CONDITIONAL_REPLACE, CONDITIONAL_REMOVE, CONTAINS_KEY, PUT_IF_ABSENT,
    REPLACE_IF_PRESENT, GET_AND_REMOVE, GET_AND_PUT, GET_ALL, PUT_ALL_DETAILED, REMOVE_ALL_DETAILED,
    SET_TTL, REMAINING_TTL, ENTRY_LISTENER, SIZE, IS_EMPTY, CONTAINS_VALUE_BOUNDED, CLEAR_DETAILED,
    DESTROY, EVICT, EVICT_ALL_DETAILED, KEY_SET_PAGE, VALUES_PAGE, ENTRY_SET_PAGE, GET_OR_DEFAULT,
    CONTAINS_ENTRY, SET, GET_ENTRY_VIEW, TRY_PUT, TRY_REMOVE, REPLACE_ALL_DETAILED,
    REMOVE_ENTRY_LISTENER, CAPABILITY_DESCRIPTOR, REPLACE_IF_VERSION, REMOVE_IF_VERSION,
    SET_TTL_IF_VERSION, QUERY_OPERATION_OUTCOME, LOCAL_STATISTICS;
    public String id() { return name().toLowerCase(java.util.Locale.ROOT); }
    public static Operation parse(String id) {
      Objects.requireNonNull(id);
      for (var operation : values()) if (operation.id().equals(id)) return operation;
      throw new IMapDomainException(IMapDomainException.Kind.UNSUPPORTED_OPERATION, "unknown development operation");
    }
  }
  public enum Mode { LOCAL_REFERENCE, PRODUCTION_MEMBER }
  public enum Failure { CONTEXT_MISMATCH, IDENTITY_MISMATCH, BLOCKED_BY_074 }
  public static final class BoundaryException extends IllegalArgumentException {
    private final Failure failure;
    BoundaryException(Failure failure) { super(failure.name()); this.failure = failure; }
    public Failure failure() { return failure; }
  }
  public record Binding(long tenant, long namespace, long generation, int partition, long epoch, long owner, String client) {
    public Binding {
      Objects.requireNonNull(client);
      if (tenant <= 0 || namespace <= 0 || generation <= 0 || partition < 0 || epoch <= 0 || owner <= 0
          || client.isBlank() || client.getBytes(StandardCharsets.UTF_8).length > 128) throw new IllegalArgumentException("invalid development binding");
    }
  }
  public static final class Prepared {
    private final Binding binding;
    private final Operation operation;
    private final UnifiedIMapService.RequestSnapshot request;
    private Prepared(Binding binding, Operation operation, UnifiedIMapService.RequestSnapshot request) {
      this.binding = binding; this.operation = operation; this.request = request;
    }
    public Operation operation() { return operation; }
    public long deadline() { return request.deadline(); }
  }
  private final Binding binding;
  private final UnifiedIMapService service;
  public DevelopmentBackendPort075(Binding binding, UnifiedIMapService service, Mode mode) {
    if (Objects.requireNonNull(mode) != Mode.LOCAL_REFERENCE) throw new BoundaryException(Failure.BLOCKED_BY_074);
    this.binding = Objects.requireNonNull(binding); this.service = Objects.requireNonNull(service);
    if (binding.generation() != service.namespaceGeneration()) throw new BoundaryException(Failure.CONTEXT_MISMATCH);
  }
  public Prepared prepare(Binding context, UnifiedIMapService.DomainCommand command) {
    if (!binding.equals(context)) throw new BoundaryException(Failure.CONTEXT_MISMATCH);
    var request = Objects.requireNonNull(command).freeze(); var operation = Operation.parse(request.operation());
    if (UnifiedIMapService.isMutation(operation.id()) && request.identity().isEmpty()
        || request.identity().isPresent() && !request.identity().orElseThrow().client().equals(binding.client()))
      throw new BoundaryException(Failure.IDENTITY_MISMATCH);
    return new Prepared(binding, operation, service.prepareRequest(request.toBuilder()));
  }
  public UnifiedIMapService.Result execute(Prepared invocation) {
    if (!binding.equals(Objects.requireNonNull(invocation).binding)) throw new BoundaryException(Failure.CONTEXT_MISMATCH);
    return service.executeSnapshot(invocation.request);
  }
}
