package io.hydracache.imap.semantic;

import java.util.List;
import java.util.Set;

/** Common W11b semantic SPI. It deliberately contains no SDK or benchmark timing types. */
public interface MapSemanticAdapter extends AutoCloseable {
  AdapterIdentity identity();
  Set<Operation> capabilities();
  OperationOutcome execute(ScenarioStep step);
  List<MapEvent> drainEvents();
  StateSnapshot snapshot();
  @Override default void close() {}
}
