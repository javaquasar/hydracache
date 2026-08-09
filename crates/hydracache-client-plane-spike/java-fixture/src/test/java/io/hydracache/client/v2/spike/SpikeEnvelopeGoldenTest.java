package io.hydracache.client.v2.spike;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;

import com.google.protobuf.ByteString;
import org.junit.jupiter.api.Test;

final class SpikeEnvelopeGoldenTest {
    private static final byte[] GOLDEN = new byte[] {
        0x08, 0x05, 0x10, 0x02, 0x18, 0x13, 0x22, 0x03, 0x67, 0x65, 0x74
    };

    @Test
    void generatedJavaCodecMatchesTheRustGoldenFrame() throws Exception {
        SpikeEnvelope envelope = SpikeEnvelope.newBuilder()
            .setGeneration(5)
            .setKind(2)
            .setCorrelationId(19)
            .setPayload(ByteString.copyFromUtf8("get"))
            .build();

        assertArrayEquals(GOLDEN, envelope.toByteArray());
        assertEquals(envelope, SpikeEnvelope.parseFrom(GOLDEN));
    }
}
