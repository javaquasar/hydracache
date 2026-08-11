import asyncio
import unittest

import grpc

from hydracache_hc2 import (
    AsyncHydraCacheClient,
    CacheEvent,
    ClientConfig,
    MutationResult,
)
from hydracache_hc2_generated import hc2_contract_pb2 as wire
from hydracache_hc2_generated import hc2_contract_pb2_grpc as wire_grpc


class TestService(wire_grpc.ClientPlaneAlphaServicer):
    def __init__(self, disconnect_first_subscription=False):
        self.connections = 0
        self.cancel_seen = asyncio.Event()
        self.disconnect_first_subscription = disconnect_first_subscription

    async def Open(self, request_iterator, context):
        self.connections += 1
        connection = self.connections
        async for request in request_iterator:
            kind = request.WhichOneof("message")
            common = {
                "generation": request.generation,
                "connection_generation": request.connection_generation,
                "correlation_id": request.correlation_id,
            }
            if kind == "handshake":
                yield wire.ServerEnvelope(
                    **common,
                    handshake=wire.HandshakeAck(
                        generation=request.generation,
                        connection_generation=request.connection_generation,
                        cluster_id="python-test-cluster",
                        accepted=request.handshake.requested,
                        minimum_generation=5,
                        preferred_generation=6,
                    ),
                )
            elif kind == "invocation":
                operation = request.invocation.WhichOneof("operation")
                if operation == "get" and request.invocation.get.key == b"cancel":
                    continue
                if operation == "get":
                    response = wire.InvocationResponse(
                        value=wire.ValueResult(found=True, value=b"value")
                    )
                elif operation == "try_lock":
                    response = wire.InvocationResponse(
                        lock=wire.LockResult(acquired=True, fence=42)
                    )
                elif operation == "lock_ownership":
                    response = wire.InvocationResponse(
                        lock_ownership=wire.LockOwnershipResult(
                            locked=True, fence=42
                        )
                    )
                else:
                    response = wire.InvocationResponse(
                        mutation=wire.MutationResult(applied=True)
                    )
                yield wire.ServerEnvelope(**common, invocation=response)
            elif kind == "cancel":
                self.cancel_seen.set()
            elif kind == "subscribe":
                subscription = request.subscribe
                yield wire.ServerEnvelope(
                    **common,
                    subscribed=wire.SubscriptionAck(
                        subscription_id=subscription.subscription_id,
                        watermark=subscription.resume_watermark,
                    ),
                )
                watermark = subscription.resume_watermark + 1
                yield wire.ServerEnvelope(
                    generation=request.generation,
                    connection_generation=request.connection_generation,
                    event=wire.CacheEvent(
                        subscription_id=subscription.subscription_id,
                        watermark=watermark,
                        key=b"key",
                        value=f"event-{watermark}".encode(),
                    ),
                )
                if self.disconnect_first_subscription and connection == 1:
                    return
            elif kind == "session_open":
                yield wire.ServerEnvelope(
                    **common,
                    session_heartbeat=wire.SessionHeartbeat(
                        session_id=b"session-1", fence=41
                    ),
                )


class AsyncClientTest(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.server = grpc.aio.server()
        self.service = TestService()
        wire_grpc.add_ClientPlaneAlphaServicer_to_server(self.service, self.server)
        self.port = self.server.add_insecure_port("127.0.0.1:0")
        await self.server.start()

    async def asyncTearDown(self):
        await self.server.stop(0)

    def config(self):
        return ClientConfig(
            endpoint=f"127.0.0.1:{self.port}",
            client_id="python-sdk-test",
            tenant="test",
            insecure=True,
            connect_timeout=2,
            request_timeout=2,
            reconnect_backoff=0.01,
        )

    async def test_data_session_and_clean_close(self):
        client = await AsyncHydraCacheClient.connect(self.config())
        try:
            self.assertEqual("python-test-cluster", client.cluster_id)
            self.assertEqual(MutationResult(True), await client.put(b"key", b"value"))
            self.assertEqual(b"value", (await client.get(b"key")).value)
            session = await client.open_session(10_000)
            self.assertEqual(41, session.fence)
            await session.close()
            acquired = await client.try_lock(b"lock", 10_000)
            self.assertEqual(42, acquired.fence)
            self.assertEqual(42, (await client.lock_ownership(b"lock")).fence)
            self.assertTrue((await client.renew_lock(b"lock", 42, 10_000)).applied)
            self.assertTrue((await client.unlock(b"lock", 42)).applied)
            self.assertEqual(0, client.metrics.pending_invocations)
            self.assertEqual(0, client.metrics.active_sessions)
        finally:
            await client.close()

    async def test_asyncio_cancellation_releases_pending_and_sends_cancel(self):
        client = await AsyncHydraCacheClient.connect(self.config())
        try:
            task = asyncio.create_task(client.get(b"cancel"))
            for _ in range(100):
                if client.metrics.pending_invocations == 1:
                    break
                await asyncio.sleep(0.01)
            task.cancel()
            with self.assertRaises(asyncio.CancelledError):
                await task
            await asyncio.wait_for(self.service.cancel_seen.wait(), 2)
            self.assertEqual(0, client.metrics.pending_invocations)
            self.assertEqual(1, client.metrics.cancelled)
        finally:
            await client.close()

    async def test_reconnect_restores_subscription_from_last_watermark(self):
        await self.server.stop(0)
        self.server = grpc.aio.server()
        self.service = TestService(disconnect_first_subscription=True)
        wire_grpc.add_ClientPlaneAlphaServicer_to_server(self.service, self.server)
        self.port = self.server.add_insecure_port("127.0.0.1:0")
        await self.server.start()
        client = await AsyncHydraCacheClient.connect(self.config())
        try:
            subscription = await client.subscribe(b"key")
            first = await asyncio.wait_for(anext(subscription), 2)
            second = await asyncio.wait_for(anext(subscription), 3)
            self.assertIsInstance(first, CacheEvent)
            self.assertIsInstance(second, CacheEvent)
            self.assertEqual((1, 2), (first.watermark, second.watermark))
            self.assertGreaterEqual(self.service.connections, 2)
            self.assertGreaterEqual(client.metrics.reconnects, 1)
            await subscription.close()
        finally:
            await client.close()


if __name__ == "__main__":
    unittest.main()
