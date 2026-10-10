package io.hydracache.imap.semantic;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

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
}
