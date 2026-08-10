import concurrent.futures
import json
import pathlib
import unittest

import grpc
import google.protobuf

import hydracache_hc2_generated
from hydracache_hc2_generated import hc2_contract_pb2 as contract
from hydracache_hc2_generated import hc2_contract_pb2_grpc as contract_grpc


GOLDEN = bytes(
    [
        0x08,
        0x05,
        0x10,
        0x07,
        0x82,
        0x01,
        0x0D,
        0x08,
        0x05,
        0x12,
        0x04,
        0x72,
        0x75,
        0x73,
        0x74,
        0x1A,
        0x01,
        0x01,
        0x20,
        0x07,
    ]
)


class EchoClientPlane(contract_grpc.ClientPlaneAlphaServicer):
    def Open(self, request_iterator, context):
        for request in request_iterator:
            yield contract.ServerEnvelope(
                generation=request.generation,
                connection_generation=request.connection_generation,
                correlation_id=request.correlation_id,
                diagnostics=contract.Diagnostics(pending_invocations=0),
            )


class GeneratedContractTest(unittest.TestCase):
    def test_package_versions_and_metadata_are_pinned(self):
        self.assertEqual("hydracache-hc2-python-1", hydracache_hc2_generated.GENERATOR_VERSION)
        self.assertEqual("6.33.4", google.protobuf.__version__)
        self.assertEqual("1.76.0", grpc.__version__)
        metadata_path = pathlib.Path(hydracache_hc2_generated.__file__).with_name(
            "contract_metadata.json"
        )
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        contract_file = next(item for item in metadata["files"] if item["name"] == "hc2_contract.proto")
        service = next(item for item in contract_file["services"] if item["name"] == "ClientPlaneAlpha")
        self.assertEqual(
            {
                "name": "Open",
                "input_type": ".hydracache.client.v2alpha.ClientEnvelope",
                "output_type": ".hydracache.client.v2alpha.ServerEnvelope",
                "client_streaming": True,
                "server_streaming": True,
            },
            service["methods"][0],
        )

    def test_python_matches_the_rust_and_java_golden(self):
        envelope = contract.ClientEnvelope(
            generation=5,
            connection_generation=7,
            handshake=contract.Handshake(
                generation=5,
                client_id="rust",
                requested=[contract.CAPABILITY_DATA],
                connection_generation=7,
            ),
        )
        self.assertEqual(GOLDEN, envelope.SerializeToString())
        self.assertEqual(envelope, contract.ClientEnvelope.FromString(GOLDEN))

    def test_unknown_additive_field_round_trips(self):
        future = bytearray(
            contract.ClientEnvelope(
                generation=5, connection_generation=7, correlation_id=9
            ).SerializeToString()
        )
        future.extend([0xFA, 0x03, 0x03, ord("n"), ord("e"), ord("w")])
        parsed = contract.ClientEnvelope.FromString(bytes(future))
        self.assertEqual(9, parsed.correlation_id)
        self.assertEqual(bytes(future), parsed.SerializeToString())

    def test_generated_bidirectional_stub_runs_on_real_loopback(self):
        server = grpc.server(concurrent.futures.ThreadPoolExecutor(max_workers=2))
        contract_grpc.add_ClientPlaneAlphaServicer_to_server(EchoClientPlane(), server)
        port = server.add_insecure_port("127.0.0.1:0")
        self.assertGreater(port, 0)
        server.start()
        channel = grpc.insecure_channel(f"127.0.0.1:{port}")
        try:
            grpc.channel_ready_future(channel).result(timeout=5)
            stub = contract_grpc.ClientPlaneAlphaStub(channel)
            requests = (
                contract.ClientEnvelope(
                    generation=5,
                    connection_generation=11,
                    correlation_id=correlation,
                )
                for correlation in range(1, 17)
            )
            responses = list(stub.Open(requests, timeout=5))
            self.assertEqual(list(range(1, 17)), [item.correlation_id for item in responses])
            self.assertTrue(all(item.HasField("diagnostics") for item in responses))
        finally:
            channel.close()
            server.stop(0).wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
