package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.HexFormat;
import org.junit.jupiter.api.Test;

final class ReferenceCanonicalKeyCodecTest {
  private static final ReferenceCanonicalKeyCodec.Bounds BOUNDS =
      ReferenceCanonicalKeyCodec.Bounds.defaults();

  @Test
  void matchesTheRustAsciiAndEmptyKeyGolden() {
    var key = new ReferenceCanonicalKeyCodec.Key("tenant-a", "orders", 1, new byte[0]);
    byte[] encoded = ReferenceCanonicalKeyCodec.encode(key, BOUNDS);
    assertEquals(
        "4843524b303735000000000874656e616e742d61000000066f7264657273000000000000000100000000",
        HexFormat.of().formatHex(encoded));
    assertEquals(key, ReferenceCanonicalKeyCodec.decode(encoded, BOUNDS));
    assertArrayEquals(new byte[0], ReferenceCanonicalKeyCodec.decode(encoded, BOUNDS).key());
  }

  @Test
  void preservesUtf8WithoutUnicodeNormalization() {
    var composed = new ReferenceCanonicalKeyCodec.Key("é", "名字", 42, new byte[] {'k'});
    var decomposed = new ReferenceCanonicalKeyCodec.Key("e\u0301", "名字", 42, new byte[] {'k'});
    String composedHex = HexFormat.of().formatHex(ReferenceCanonicalKeyCodec.encode(composed, BOUNDS));
    assertEquals(
        "4843524b3037350000000002c3a900000006e5908de5ad97000000000000002a000000016b",
        composedHex);
    assertNotEquals(composedHex,
        HexFormat.of().formatHex(ReferenceCanonicalKeyCodec.encode(decomposed, BOUNDS)));
  }

  @Test
  void rejectsUnsignedHostileLengthBeforeAllocation() {
    byte[] malicious = HexFormat.of().parseHex("4843524b30373500ffffffff");
    assertThrows(IllegalArgumentException.class,
        () -> ReferenceCanonicalKeyCodec.decode(malicious,
            new ReferenceCanonicalKeyCodec.Bounds(4, 4, 4, 64)));
  }

  @Test
  void matchesTheRustBinaryKeyGoldenAtTheMaxPositiveJavaGeneration() {
    byte[] encoded = HexFormat.of().parseHex(
        "4843524b303735000000000874656e616e742d61000000066f72646572737fffffffffffffff0000000400ff8000");
    var key = new ReferenceCanonicalKeyCodec.Key("tenant-a", "orders", Long.MAX_VALUE,
        new byte[] {0, (byte) 0xff, (byte) 0x80, 0});
    assertArrayEquals(encoded, ReferenceCanonicalKeyCodec.encode(key, BOUNDS));
    assertEquals(key, ReferenceCanonicalKeyCodec.decode(encoded, BOUNDS));
  }

  @Test
  void rejectsTruncatedBinaryFramesTrailingDataAndUnknownDomainVersions() {
    var key = new ReferenceCanonicalKeyCodec.Key("tenant-a", "orders", 1,
        new byte[] {0, (byte) 0xff, (byte) 0x80, 0});
    byte[] frame = ReferenceCanonicalKeyCodec.encode(key, BOUNDS);
    for (int length = 0; length < frame.length; length++) {
      byte[] truncated = Arrays.copyOf(frame, length);
      assertThrows(IllegalArgumentException.class,
          () -> ReferenceCanonicalKeyCodec.decode(truncated, BOUNDS),
          "accepted a truncated frame at " + length + " bytes");
    }
    byte[] trailing = Arrays.copyOf(frame, frame.length + 1);
    assertThrows(IllegalArgumentException.class,
        () -> ReferenceCanonicalKeyCodec.decode(trailing, BOUNDS));
    frame[6] = '6';
    assertThrows(IllegalArgumentException.class,
        () -> ReferenceCanonicalKeyCodec.decode(frame, BOUNDS));
  }

  @Test
  void matchesTheSharedProvisionalFnvAndUnsignedPartitionVectors() throws IOException {
    Path relative = Path.of("docs/testing/imap/0.75/partition-hash-fnv1a64-vectors.tsv");
    Path root = Path.of("").toAbsolutePath();
    while (root != null && !Files.isRegularFile(root.resolve(relative))) {
      root = root.getParent();
    }
    assertNotNull(root, "canonical partition fixture");
    var rows = Files.readAllLines(root.resolve(relative));
    assertEquals(6, rows.size());
    for (String row : rows.subList(1, rows.size())) {
      String[] fields = row.split("\t");
      byte[] frame = HexFormat.of().parseHex(fields[1]);
      var key = ReferenceCanonicalKeyCodec.decode(frame, BOUNDS);
      assertArrayEquals(frame, ReferenceCanonicalKeyCodec.encode(key, BOUNDS), fields[0]);
      long hash = 0xcbf29ce484222325L;
      for (byte value : frame) {
        hash = (hash ^ Byte.toUnsignedLong(value)) * 0x100000001b3L;
      }
      assertEquals(Long.parseUnsignedLong(fields[2], 16), hash, fields[0]);
      assertEquals(Long.parseLong(fields[3]), Long.remainderUnsigned(hash, 271), fields[0]);
    }
  }
}
