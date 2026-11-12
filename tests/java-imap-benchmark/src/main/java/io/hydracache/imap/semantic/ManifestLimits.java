package io.hydracache.imap.semantic;

/** Hard parser limits, enforced before decoded payload allocation. */
public record ManifestLimits(
    int maxCharacters,
    int maxSteps,
    int maxKeyBytes,
    int maxValueBytes,
    int maxBulkItems,
    int maxTotalPayloadBytes,
    int maxLineCharacters) {
  public ManifestLimits {
    if (maxCharacters <= 0 || maxSteps <= 0 || maxKeyBytes <= 0 || maxValueBytes <= 0
        || maxBulkItems <= 0 || maxTotalPayloadBytes <= 0 || maxLineCharacters <= 0) {
      throw new IllegalArgumentException("all manifest limits must be positive");
    }
  }

  public static ManifestLimits defaults() {
    return new ManifestLimits(64 * 1024, 1024, 1024, 1024 * 1024, 256, 8 * 1024 * 1024, 8192);
  }
}
