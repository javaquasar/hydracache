package io.hydracache.imap.semantic;

import java.util.List;
import java.util.Set;

abstract class BlockedProductAdapter implements MapSemanticAdapter {
  private final AdapterIdentity identity;

  BlockedProductAdapter(String product) {
    identity = new AdapterIdentity(product, "unbound-product-adapter", "blocked-by-0.74");
  }

  @Override public final AdapterIdentity identity() { return identity; }
  @Override public final Set<Operation> capabilities() { return Set.of(); }

  @Override
  public final OperationOutcome execute(ScenarioStep step) {
    throw new AdapterUnavailableException(identity.product()
        + " adapter is blocked-by-0.74 until packaged product APIs and identities are frozen");
  }

  @Override public final List<MapEvent> drainEvents() { return List.of(); }

  @Override
  public final StateSnapshot snapshot() {
    throw new AdapterUnavailableException(identity.product()
        + " adapter is blocked-by-0.74 and has no authoritative product state");
  }
}
