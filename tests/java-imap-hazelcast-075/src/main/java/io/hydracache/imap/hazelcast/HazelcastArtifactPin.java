package io.hydracache.imap.hazelcast;

import com.hazelcast.core.Hazelcast;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;

/** Independent GA artifact pin; the reviewed development source map has a different identity. */
public final class HazelcastArtifactPin {
  public static final String VERSION = "5.5.0";
  public static final String SOURCE_SHA = "e6ef7a8a874e445754bf6b9ab04056d812d993da";
  public static final String JAR_SHA256 = "cf560b40b988526e780e4fae12cb6fe01c5158bb9e00839882857f32b544a19e";
  public static final String POM_SHA256 = "b84a92ffc6798bcf0e94f38df11c4445e2a9a4f6fd31082a057b179ee404db23";
  private HazelcastArtifactPin() {}

  public static String verifyLoadedArtifact() {
    requireVersion(Hazelcast.class.getPackage().getImplementationVersion());
    return verifyJar(Path.of(java.net.URI.create(Hazelcast.class.getProtectionDomain().getCodeSource().getLocation().toExternalForm())));
  }
  static void requireVersion(String version) {
    if (!VERSION.equals(version))
      throw new IllegalStateException("unexpected Hazelcast implementation version");
  }

  public static String verifyJar(Path jar) {
    try {
      long size = Files.size(jar);
      if (!Files.isRegularFile(jar) || Files.isSymbolicLink(jar) || size <= 0 || size > 32L * 1024 * 1024)
        throw new IllegalStateException("Hazelcast artifact bound/type");
      MessageDigest digest = MessageDigest.getInstance("SHA-256");
      try (var input = Files.newInputStream(jar)) {
        byte[] buffer = new byte[32768]; int count; long read = 0;
        while ((count = input.read(buffer)) != -1) {
          read += count; if (read > size) throw new IllegalStateException("Hazelcast artifact changed during hashing");
          digest.update(buffer, 0, count);
        }
        if (read != size) throw new IllegalStateException("Hazelcast artifact truncated during hashing");
      }
      String actual = HexFormat.of().formatHex(digest.digest());
      if (!JAR_SHA256.equals(actual)) throw new IllegalStateException("Hazelcast artifact checksum mismatch");
      return actual;
    } catch (IOException | NoSuchAlgorithmException failure) { throw new IllegalStateException("cannot verify Hazelcast artifact", failure); }
  }
}
