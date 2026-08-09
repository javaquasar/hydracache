package io.hydracache.client.v2alpha;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.nio.charset.StandardCharsets;
import java.security.KeyFactory;
import java.security.PublicKey;
import java.security.Signature;
import java.security.spec.X509EncodedKeySpec;
import org.junit.jupiter.api.Test;

final class SignedDiscoveryVectorTest {
  private static final String CANONICAL_HEX =
      "485944524143414348452d4843322d444953434f564552590000010001000b726f6f742d323032362d61000000006b49d200000000006b49e0100009636c75737465722d61000000000000002a0005000100066e6f64652d6100000000000000070101020300226863322b677270633a2f2f6e6f64652d612e6578616d706c652e636f6d3a3734343300126e6f64652d612e6578616d706c652e636f6d";
  private static final String PUBLIC_KEY_HEX =
      "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c";
  private static final String SIGNATURE_HEX =
      "2bcce116ff87d015d8f36564a85044df33adecca281c9d6a38355fc28d5edac9ca8b2a9314c2d3ff1244dff1c30b648af279959bfa72ff4705a9dd3276f5780c";
  private static final String ED25519_SPKI_PREFIX_HEX = "302a300506032b6570032100";

  @Test
  void java_reconstructs_and_verifies_the_rust_ed25519_vector() throws Exception {
    byte[] canonical = canonicalBytes();
    assertArrayEquals(hex(CANONICAL_HEX), canonical);

    byte[] spki = concat(hex(ED25519_SPKI_PREFIX_HEX), hex(PUBLIC_KEY_HEX));
    PublicKey publicKey =
        KeyFactory.getInstance("Ed25519").generatePublic(new X509EncodedKeySpec(spki));
    Signature verifier = Signature.getInstance("Ed25519");
    verifier.initVerify(publicKey);
    verifier.update(canonical);
    assertTrue(verifier.verify(hex(SIGNATURE_HEX)));
  }

  private static byte[] canonicalBytes() throws Exception {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    try (DataOutputStream output = new DataOutputStream(bytes)) {
      output.write("HYDRACACHE-HC2-DISCOVERY\0".getBytes(StandardCharsets.US_ASCII));
      output.writeShort(1); // format
      output.writeShort(1); // Ed25519
      writeString(output, "root-2026-a");
      output.writeLong(1_800_000_000L);
      output.writeLong(1_800_003_600L);
      writeString(output, "cluster-a");
      output.writeLong(42L);
      output.writeShort(5); // HC/2 generation
      output.writeShort(1); // nodes
      writeString(output, "node-a");
      output.writeLong(7L);
      output.writeByte(1); // endpoints
      output.writeByte(1); // gRPC bidi
      output.writeByte(2); // preview
      output.writeByte(3); // ready + mTLS
      writeString(output, "hc2+grpc://node-a.example.com:7443");
      writeString(output, "node-a.example.com");
    }
    return bytes.toByteArray();
  }

  private static void writeString(DataOutputStream output, String value) throws Exception {
    byte[] encoded = value.getBytes(StandardCharsets.US_ASCII);
    output.writeShort(encoded.length);
    output.write(encoded);
  }

  private static byte[] hex(String value) {
    return java.util.HexFormat.of().parseHex(value);
  }

  private static byte[] concat(byte[] left, byte[] right) {
    byte[] combined = new byte[left.length + right.length];
    System.arraycopy(left, 0, combined, 0, left.length);
    System.arraycopy(right, 0, combined, left.length, right.length);
    return combined;
  }
}
