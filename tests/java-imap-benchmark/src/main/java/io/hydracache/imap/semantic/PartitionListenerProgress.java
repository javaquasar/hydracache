package io.hydracache.imap.semantic;

/** Generation-aware per-partition listener watermark with explicit gap repair. */
public final class PartitionListenerProgress {
  public enum Phase { STREAMING, GAP_DETECTED, REPAIRING }

  private final int partition;
  private long generation;
  private long watermark;
  private Phase phase = Phase.STREAMING;

  public PartitionListenerProgress(int partition, long generation, long watermark) {
    if (partition < 0 || generation <= 0 || watermark < 0) {
      throw new IllegalArgumentException("invalid listener cursor");
    }
    this.partition = partition;
    this.generation = generation;
    this.watermark = watermark;
  }

  public int partition() { return partition; }
  public long generation() { return generation; }
  public long watermark() { return watermark; }
  public Phase phase() { return phase; }

  public boolean accept(long eventGeneration, long sequence) {
    if (eventGeneration < generation) {
      throw new SemanticMismatchException("stale listener generation");
    }
    if (phase != Phase.STREAMING) {
      throw new SemanticMismatchException("listener delivery crossed an open gap");
    }
    if (eventGeneration > generation || sequence > watermark + 1) {
      generation = eventGeneration;
      phase = Phase.GAP_DETECTED;
      return false;
    }
    if (sequence <= watermark) return false;
    watermark = sequence;
    return true;
  }

  public void beginRepair() {
    if (phase != Phase.GAP_DETECTED) {
      throw new SemanticMismatchException("listener repair requires a gap");
    }
    phase = Phase.REPAIRING;
  }

  public void completeRepair(long repairGeneration, long snapshotWatermark, long cutover) {
    if (phase != Phase.REPAIRING || repairGeneration != generation
        || snapshotWatermark < watermark || cutover < snapshotWatermark) {
      throw new SemanticMismatchException("listener repair does not close the generation gap");
    }
    watermark = cutover;
    phase = Phase.STREAMING;
  }
}
