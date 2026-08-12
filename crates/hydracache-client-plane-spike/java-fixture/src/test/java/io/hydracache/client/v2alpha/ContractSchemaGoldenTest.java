package io.hydracache.client.v2alpha;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;

import com.google.protobuf.UnknownFieldSet;
import io.hydracache.client.hc2.internal.wire.Capability;
import io.hydracache.client.hc2.internal.wire.ClientEnvelope;
import io.hydracache.client.hc2.internal.wire.Handshake;
import io.hydracache.client.hc2.internal.wire.InvocationRequest;
import java.io.ByteArrayOutputStream;
import org.junit.jupiter.api.Test;

final class ContractSchemaGoldenTest {
    private static final byte[] GOLDEN = new byte[] {
        0x08, 0x05, 0x10, 0x07, (byte) 0x82, 0x01, 0x0d, 0x08, 0x05, 0x12, 0x04,
        0x72, 0x75, 0x73, 0x74, 0x1a, 0x01, 0x01, 0x20, 0x07
    };

    @Test
    void generatedJavaContractMatchesRustGoldenAndStableIds() throws Exception {
        ClientEnvelope envelope = ClientEnvelope.newBuilder()
            .setGeneration(5)
            .setConnectionGeneration(7)
            .setHandshake(Handshake.newBuilder()
                .setGeneration(5)
                .setClientId("rust")
                .addRequested(Capability.CAPABILITY_DATA)
                .setConnectionGeneration(7))
            .build();

        assertArrayEquals(GOLDEN, envelope.toByteArray());
        assertEquals(envelope, ClientEnvelope.parseFrom(GOLDEN));
        assertEquals(101, InvocationRequest.getDescriptor().findFieldByName("get").getNumber());
        assertEquals(105, InvocationRequest.getDescriptor().findFieldByName("batch").getNumber());
        assertEquals(16, ClientEnvelope.getDescriptor().findFieldByName("handshake").getNumber());
        assertEquals(21, ClientEnvelope.getDescriptor().findFieldByName("session_heartbeat").getNumber());
    }

    @Test
    void JavaRuntimePreservesUnknownAdditiveFields() throws Exception {
        ByteArrayOutputStream future = new ByteArrayOutputStream();
        future.write(ClientEnvelope.newBuilder().setGeneration(5).setCorrelationId(9).build().toByteArray());
        future.write(new byte[] {(byte) 0xfa, 0x03, 0x03, 'n', 'e', 'w'});

        ClientEnvelope parsed = ClientEnvelope.parseFrom(future.toByteArray());
        UnknownFieldSet.Field unknown = parsed.getUnknownFields().getField(63);
        assertEquals("new", unknown.getLengthDelimitedList().get(0).toStringUtf8());
        assertArrayEquals(future.toByteArray(), parsed.toByteArray());
    }
}
