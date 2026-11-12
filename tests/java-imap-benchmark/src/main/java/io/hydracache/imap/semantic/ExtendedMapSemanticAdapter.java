package io.hydracache.imap.semantic;

import java.util.Set;
import static io.hydracache.imap.semantic.ExtendedSemanticScenario.*;

/** Common development-only v2 SPI; implementations disclose capabilities before executing. */
public interface ExtendedMapSemanticAdapter extends AutoCloseable {
  AdapterIdentity identity();
  Set<Op> extendedCapabilities();
  Observation executeExtended(Step step);
  @Override void close();
}
