package io.hydracache.imap.semantic;

/** Logical sequence fence for listener gap detection, snapshot repair, and stream cutover. */
public final class ListenerRepairBoundary {
  public enum Phase { STREAMING, GAP_DETECTED, REPAIRING }

  private Phase phase = Phase.STREAMING;
  private long lastDelivered;
  private long cutover;

  public Phase phase() { return phase; }
  public long lastDelivered() { return lastDelivered; }

  public void accept(long sequence) {
    if (phase != Phase.STREAMING || sequence <= Math.max(lastDelivered, cutover)) {
      throw new SemanticMismatchException("listener event crossed a gap/cutover fence");
    }
    lastDelivered = sequence;
  }

  public void gap(long expectedNext, long observed) {
    if (phase != Phase.STREAMING || expectedNext != lastDelivered + 1 || observed <= expectedNext) {
      throw new SemanticMismatchException("invalid listener gap evidence");
    }
    phase = Phase.GAP_DETECTED;
  }

  public void beginRepair() {
    if (phase != Phase.GAP_DETECTED) {
      throw new SemanticMismatchException("repair must follow a detected gap");
    }
    phase = Phase.REPAIRING;
  }

  public void completeRepair(long snapshotWatermark, long streamCutover) {
    if (phase != Phase.REPAIRING || snapshotWatermark > streamCutover
        || streamCutover < lastDelivered) {
      throw new SemanticMismatchException("repair watermarks do not close the gap");
    }
    cutover = streamCutover;
    lastDelivered = streamCutover;
    phase = Phase.STREAMING;
  }
}
