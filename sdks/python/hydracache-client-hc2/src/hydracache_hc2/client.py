from __future__ import annotations

import asyncio
import contextlib
import time
from pathlib import Path
from typing import AsyncIterator, Optional

import grpc

from hydracache_hc2_generated import hc2_contract_pb2 as wire
from hydracache_hc2_generated import hc2_contract_pb2_grpc as wire_grpc

from .errors import ErrorCode, HydraCacheError, RetryAdvice
from .models import (
    CacheEvent,
    CacheValue,
    ClientConfig,
    ClientMetrics,
    EventGap,
    MutationResult,
    RequestOptions,
)

_END = object()


class Subscription:
    def __init__(
        self,
        client: "AsyncHydraCacheClient",
        subscription_id: int,
        queue: asyncio.Queue[object],
    ):
        self._client = client
        self.subscription_id = subscription_id
        self._queue = queue
        self._closed = False

    def __aiter__(self) -> "Subscription":
        return self

    async def __anext__(self) -> CacheEvent | EventGap:
        item = await self._queue.get()
        if item is _END:
            raise StopAsyncIteration
        if isinstance(item, Exception):
            raise item
        return item  # type: ignore[return-value]

    async def close(self) -> None:
        if not self._closed:
            self._closed = True
            await self._client._unsubscribe(self.subscription_id)


class FencedSession:
    def __init__(
        self, client: "AsyncHydraCacheClient", session_id: bytes, fence: int
    ):
        self._client = client
        self.session_id = session_id
        self.fence = fence
        self.lost = False
        self._closed = False

    async def heartbeat(self) -> int:
        if self._closed or self.lost:
            raise HydraCacheError(
                ErrorCode.SESSION_LOST,
                RetryAdvice.NEVER,
                "fenced session is not active",
            )
        await self._client._session_heartbeat(self)
        return self.fence

    async def close(self) -> None:
        if not self._closed:
            self._closed = True
            await self._client._session_close(self)


class AsyncHydraCacheClient:
    """Bounded asyncio HC/2 client over one reconnecting bidirectional stream."""

    def __init__(self, config: ClientConfig):
        config.validate()
        self._config = config
        self._closed = False
        self._terminal_error: Optional[HydraCacheError] = None
        self._ready = asyncio.Event()
        self._initial = asyncio.get_running_loop().create_future()
        self._outbound: Optional[asyncio.Queue[object]] = None
        self._channel: Optional[grpc.aio.Channel] = None
        self._generation = 0
        self._correlation = 0
        self._subscription_id = 0
        self._pending: dict[int, asyncio.Future[wire.InvocationResponse]] = {}
        self._pending_subscriptions: dict[
            int, asyncio.Future[wire.SubscriptionAck]
        ] = {}
        self._pending_sessions: dict[int, asyncio.Future[FencedSession]] = {}
        self._subscriptions: dict[
            int, tuple[bytes, int, asyncio.Queue[object]]
        ] = {}
        self._sessions: dict[bytes, FencedSession] = {}
        self._invocation_slots = asyncio.Semaphore(
            config.limits.max_pending_invocations
        )
        self._subscription_slots = asyncio.Semaphore(config.limits.max_subscriptions)
        self._session_slots = asyncio.Semaphore(config.limits.max_sessions)
        self._submitted = 0
        self._completed = 0
        self._failed = 0
        self._cancelled = 0
        self._reconnects = 0
        self.cluster_id = ""
        self.preferred_protocol_generation = config.protocol_generation
        self.negotiated_generation_deprecated = False
        self._runner = asyncio.create_task(
            self._connection_loop(), name="hydracache-hc2-connection"
        )

    @classmethod
    async def connect(cls, config: ClientConfig) -> "AsyncHydraCacheClient":
        client = cls(config)
        try:
            await client._initial
            return client
        except BaseException:
            await client.close()
            raise

    @property
    def metrics(self) -> ClientMetrics:
        return ClientMetrics(
            submitted=self._submitted,
            completed=self._completed,
            failed=self._failed,
            cancelled=self._cancelled,
            reconnects=self._reconnects,
            pending_invocations=len(self._pending),
            active_subscriptions=len(self._subscriptions),
            active_sessions=len(self._sessions),
        )

    async def get(
        self, key: bytes, options: Optional[RequestOptions] = None
    ) -> Optional[CacheValue]:
        _bytes(key, "key", 1, 1024 * 1024)
        response = await self._invoke(
            wire.InvocationRequest(get=wire.GetRequest(key=key)), options
        )
        if not response.HasField("value") or not response.value.found:
            return None
        expires = response.value.expires_at_unix_ms or None
        return CacheValue(bytes(response.value.value), expires)

    async def put(
        self,
        key: bytes,
        value: bytes,
        ttl_ms: int = 0,
        options: Optional[RequestOptions] = None,
    ) -> MutationResult:
        _bytes(key, "key", 1, 1024 * 1024)
        _bytes(value, "value", 0, 16 * 1024 * 1024)
        _ttl(ttl_ms)
        response = await self._invoke(
            wire.InvocationRequest(
                put=wire.PutRequest(key=key, value=value, ttl_ms=ttl_ms)
            ),
            options,
        )
        return _mutation(response)

    async def delete(
        self, key: bytes, options: Optional[RequestOptions] = None
    ) -> MutationResult:
        _bytes(key, "key", 1, 1024 * 1024)
        response = await self._invoke(
            wire.InvocationRequest(delete=wire.DeleteRequest(key=key)), options
        )
        return _mutation(response)

    async def compare_and_set(
        self,
        key: bytes,
        expected: bytes,
        replacement: bytes,
        ttl_ms: int = 0,
        options: Optional[RequestOptions] = None,
    ) -> MutationResult:
        _bytes(key, "key", 1, 1024 * 1024)
        _bytes(expected, "expected", 0, 16 * 1024 * 1024)
        _bytes(replacement, "replacement", 0, 16 * 1024 * 1024)
        _ttl(ttl_ms)
        request = wire.CompareAndSetRequest(
            key=key, expected=expected, replacement=replacement, ttl_ms=ttl_ms
        )
        response = await self._invoke(
            wire.InvocationRequest(compare_and_set=request), options
        )
        return _mutation(response)

    async def subscribe(
        self, key_prefix: bytes = b"", resume_watermark: int = 0
    ) -> Subscription:
        _bytes(key_prefix, "key_prefix", 0, 1024 * 1024)
        if resume_watermark < 0:
            raise ValueError("resume_watermark must be nonnegative")
        await self._wait_ready()
        await self._subscription_slots.acquire()
        self._subscription_id += 1
        subscription_id = self._subscription_id
        queue: asyncio.Queue[object] = asyncio.Queue(
            self._config.limits.max_subscription_events
        )
        self._subscriptions[subscription_id] = (
            key_prefix,
            resume_watermark,
            queue,
        )
        try:
            await self._send_subscription(
                subscription_id, key_prefix, resume_watermark
            )
        except BaseException:
            self._subscriptions.pop(subscription_id, None)
            self._subscription_slots.release()
            raise
        return Subscription(self, subscription_id, queue)

    async def open_session(self, requested_ttl_ms: int) -> FencedSession:
        if not 0 < requested_ttl_ms <= 300_000:
            raise ValueError("session TTL must be in [1, 300000] ms")
        await self._wait_ready()
        await self._session_slots.acquire()
        correlation = self._next_correlation()
        future: asyncio.Future[FencedSession] = (
            asyncio.get_running_loop().create_future()
        )
        self._pending_sessions[correlation] = future
        try:
            await self._enqueue(
                wire.ClientEnvelope(
                    generation=self._config.protocol_generation,
                    connection_generation=self._generation,
                    correlation_id=correlation,
                    session_open=wire.SessionOpen(
                        requested_ttl_ms=requested_ttl_ms
                    ),
                )
            )
            return await asyncio.wait_for(
                asyncio.shield(future), self._config.request_timeout
            )
        except BaseException:
            self._pending_sessions.pop(correlation, None)
            self._session_slots.release()
            raise

    async def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        self._ready.clear()
        if self._outbound is not None:
            with contextlib.suppress(asyncio.QueueFull):
                self._outbound.put_nowait(_END)
        self._runner.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await self._runner
        if self._channel is not None:
            await self._channel.close()
        self._fail_pending(_unavailable("client is closed"))
        for _, _, queue in tuple(self._subscriptions.values()):
            _queue_terminal(queue, _END)
            self._subscription_slots.release()
        self._subscriptions.clear()
        for session in self._sessions.values():
            session.lost = True
            self._session_slots.release()
        self._sessions.clear()

    async def _connection_loop(self) -> None:
        attempts = 0
        while not self._closed:
            try:
                await self._connected_stream()
                raise _unavailable("HC/2 stream ended")
            except asyncio.CancelledError:
                raise
            except BaseException:
                self._ready.clear()
                self._fail_pending(_unavailable("HC/2 connection was lost"))
                if self._closed:
                    return
                attempts += 1
                if attempts > self._config.reconnect_max_attempts:
                    error = _unavailable("HC/2 reconnect budget exhausted")
                    self._terminal_error = error
                    if not self._initial.done():
                        self._initial.set_exception(error)
                    for _, _, queue in self._subscriptions.values():
                        _queue_terminal(queue, error)
                    return
                if self._initial.done():
                    self._reconnects += 1
                await asyncio.sleep(self._config.reconnect_backoff * attempts)

    async def _connected_stream(self) -> None:
        self._generation += 1
        channel = self._new_channel()
        self._channel = channel
        await asyncio.wait_for(
            channel.channel_ready(), self._config.connect_timeout
        )
        outbound: asyncio.Queue[object] = asyncio.Queue(
            self._config.limits.max_outbound_frames
        )
        self._outbound = outbound
        call = wire_grpc.ClientPlaneAlphaStub(channel).Open(
            self._requests(outbound)
        )
        handshake: asyncio.Future[wire.HandshakeAck] = (
            asyncio.get_running_loop().create_future()
        )
        correlation = self._next_correlation()
        reader = asyncio.create_task(
            self._read_stream(call, correlation, handshake)
        )
        try:
            await self._enqueue(
                wire.ClientEnvelope(
                    generation=self._config.protocol_generation,
                    connection_generation=self._generation,
                    correlation_id=correlation,
                    handshake=wire.Handshake(
                        generation=self._config.protocol_generation,
                        client_id=self._config.client_id,
                        requested=[int(item) for item in self._config.capabilities],
                        connection_generation=self._generation,
                    ),
                )
            )
            ack = await asyncio.wait_for(
                asyncio.shield(handshake), self._config.connect_timeout
            )
            self._validate_handshake(ack)
            self._ready.set()
            if not self._initial.done():
                self._initial.set_result(None)
            await self._restore_subscriptions()
            await reader
        finally:
            reader.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await reader
            if self._outbound is outbound:
                self._outbound = None
            await channel.close()

    async def _read_stream(
        self, call, handshake_correlation: int, handshake
    ) -> None:
        try:
            async for response in call:
                if (
                    response.generation != self._config.protocol_generation
                    or response.connection_generation != self._generation
                ):
                    continue
                kind = response.WhichOneof("message")
                if (
                    kind == "handshake"
                    and response.correlation_id == handshake_correlation
                ):
                    if not handshake.done():
                        handshake.set_result(response.handshake)
                elif kind == "invocation":
                    pending = self._pending.pop(response.correlation_id, None)
                    if pending is not None and not pending.done():
                        pending.set_result(response.invocation)
                elif kind == "subscribed":
                    pending = self._pending_subscriptions.pop(
                        response.correlation_id, None
                    )
                    if pending is not None and not pending.done():
                        pending.set_result(response.subscribed)
                elif kind == "event":
                    self._deliver_event(response.event)
                elif kind == "gap":
                    self._deliver_gap(response.gap)
                elif kind == "session_heartbeat":
                    self._deliver_session(
                        response.correlation_id, response.session_heartbeat
                    )
                elif kind == "session_lost":
                    session = self._sessions.pop(
                        bytes(response.session_lost.session_id), None
                    )
                    if session is not None:
                        session.lost = True
                        self._session_slots.release()
        except BaseException as error:
            if not handshake.done():
                handshake.set_exception(
                    _unavailable("HC/2 handshake stream failed")
                )
            raise error
        finally:
            if not handshake.done():
                handshake.set_exception(
                    _unavailable("HC/2 handshake stream ended")
                )

    async def _requests(
        self, queue: asyncio.Queue[object]
    ) -> AsyncIterator[wire.ClientEnvelope]:
        while True:
            item = await queue.get()
            if item is _END:
                return
            yield item  # type: ignore[misc]

    async def _invoke(
        self, operation: wire.InvocationRequest, options: Optional[RequestOptions]
    ) -> wire.InvocationResponse:
        options = options or RequestOptions()
        timeout = options.validate(self._config.request_timeout)
        await self._wait_ready()
        await self._invocation_slots.acquire()
        correlation = self._next_correlation()
        future: asyncio.Future[wire.InvocationResponse] = (
            asyncio.get_running_loop().create_future()
        )
        operation.meta.CopyFrom(
            wire.RequestMeta(
                deadline_unix_ms=int((time.time() + timeout) * 1000),
                idempotency_key=options.idempotency_key,
                tenant=self._config.tenant,
            )
        )
        self._pending[correlation] = future
        self._submitted += 1
        try:
            await self._enqueue(
                wire.ClientEnvelope(
                    generation=self._config.protocol_generation,
                    connection_generation=self._generation,
                    correlation_id=correlation,
                    invocation=operation,
                )
            )
            response = await asyncio.wait_for(asyncio.shield(future), timeout)
            _raise_response_error(response.meta)
            self._completed += 1
            return response
        except asyncio.CancelledError:
            self._cancelled += 1
            self._pending.pop(correlation, None)
            await self._best_effort_cancel(correlation, "caller cancelled")
            raise
        except asyncio.TimeoutError as error:
            self._failed += 1
            self._pending.pop(correlation, None)
            await self._best_effort_cancel(correlation, "deadline exceeded")
            raise HydraCacheError(
                ErrorCode.DEADLINE_EXCEEDED,
                RetryAdvice.NEVER,
                "request deadline exceeded",
            ) from error
        except BaseException:
            self._failed += 1
            self._pending.pop(correlation, None)
            raise
        finally:
            self._invocation_slots.release()

    async def _best_effort_cancel(self, correlation: int, reason: str) -> None:
        if self._outbound is None:
            return
        with contextlib.suppress(asyncio.QueueFull):
            self._outbound.put_nowait(
                wire.ClientEnvelope(
                    generation=self._config.protocol_generation,
                    connection_generation=self._generation,
                    correlation_id=self._next_correlation(),
                    cancel=wire.Cancel(
                        correlation_id=correlation, safe_reason=reason
                    ),
                )
            )

    async def _send_subscription(
        self, subscription_id: int, prefix: bytes, watermark: int
    ) -> None:
        correlation = self._next_correlation()
        future: asyncio.Future[wire.SubscriptionAck] = (
            asyncio.get_running_loop().create_future()
        )
        self._pending_subscriptions[correlation] = future
        try:
            await self._enqueue(
                wire.ClientEnvelope(
                    generation=self._config.protocol_generation,
                    connection_generation=self._generation,
                    correlation_id=correlation,
                    subscribe=wire.Subscribe(
                        subscription_id=subscription_id,
                        key_prefix=prefix,
                        resume_watermark=watermark,
                    ),
                )
            )
            ack = await asyncio.wait_for(
                asyncio.shield(future), self._config.request_timeout
            )
            if ack.subscription_id != subscription_id:
                raise _unavailable(
                    "peer acknowledged a different subscription"
                )
        finally:
            self._pending_subscriptions.pop(correlation, None)

    async def _restore_subscriptions(self) -> None:
        for subscription_id, (prefix, watermark, _) in tuple(
            self._subscriptions.items()
        ):
            await self._send_subscription(subscription_id, prefix, watermark)

    def _deliver_event(self, event: wire.CacheEvent) -> None:
        current = self._subscriptions.get(event.subscription_id)
        if current is None:
            return
        prefix, watermark, queue = current
        if event.watermark <= watermark:
            return
        self._subscriptions[event.subscription_id] = (
            prefix,
            event.watermark,
            queue,
        )
        item = CacheEvent(
            event.subscription_id,
            event.watermark,
            bytes(event.key),
            bytes(event.value),
            event.removed,
        )
        if queue.full():
            _queue_terminal(queue, EventGap(event.subscription_id, watermark))
        else:
            queue.put_nowait(item)

    def _deliver_gap(self, gap: wire.EventGap) -> None:
        current = self._subscriptions.get(gap.subscription_id)
        if current is not None:
            _queue_terminal(
                current[2], EventGap(gap.subscription_id, gap.after_watermark)
            )

    def _deliver_session(
        self, correlation: int, heartbeat: wire.SessionHeartbeat
    ) -> None:
        pending = self._pending_sessions.pop(correlation, None)
        session_id = bytes(heartbeat.session_id)
        if pending is not None and not pending.done():
            session = FencedSession(self, session_id, heartbeat.fence)
            self._sessions[session_id] = session
            pending.set_result(session)
        elif session_id in self._sessions:
            self._sessions[session_id].fence = heartbeat.fence

    async def _unsubscribe(self, subscription_id: int) -> None:
        current = self._subscriptions.pop(subscription_id, None)
        if current is None:
            return
        self._subscription_slots.release()
        _queue_terminal(current[2], _END)
        if self._ready.is_set():
            await self._enqueue(
                wire.ClientEnvelope(
                    generation=self._config.protocol_generation,
                    connection_generation=self._generation,
                    correlation_id=self._next_correlation(),
                    unsubscribe=wire.Unsubscribe(subscription_id=subscription_id),
                )
            )

    async def _session_heartbeat(self, session: FencedSession) -> None:
        await self._wait_ready()
        await self._enqueue(
            wire.ClientEnvelope(
                generation=self._config.protocol_generation,
                connection_generation=self._generation,
                correlation_id=self._next_correlation(),
                session_heartbeat=wire.SessionHeartbeat(
                    session_id=session.session_id, fence=session.fence
                ),
            )
        )

    async def _session_close(self, session: FencedSession) -> None:
        if self._sessions.pop(session.session_id, None) is not None:
            self._session_slots.release()
        if self._ready.is_set():
            await self._enqueue(
                wire.ClientEnvelope(
                    generation=self._config.protocol_generation,
                    connection_generation=self._generation,
                    correlation_id=self._next_correlation(),
                    session_close=wire.SessionClose(
                        session_id=session.session_id, fence=session.fence
                    ),
                )
            )

    async def _wait_ready(self) -> None:
        if self._terminal_error is not None:
            raise self._terminal_error
        if self._closed:
            raise _unavailable("client is closed")
        try:
            await asyncio.wait_for(
                self._ready.wait(), self._config.request_timeout
            )
        except asyncio.TimeoutError as error:
            raise _unavailable("HC/2 connection is not ready") from error

    async def _enqueue(self, envelope: wire.ClientEnvelope) -> None:
        if self._outbound is None:
            raise _unavailable("HC/2 connection is not ready")
        try:
            await asyncio.wait_for(
                self._outbound.put(envelope), self._config.request_timeout
            )
        except asyncio.TimeoutError as error:
            raise HydraCacheError(
                ErrorCode.QUOTA_EXCEEDED,
                RetryAdvice.NEVER,
                "outbound queue is full",
            ) from error

    def _new_channel(self) -> grpc.aio.Channel:
        options = [
            (
                "grpc.max_receive_message_length",
                self._config.limits.max_inbound_message_bytes,
            )
        ]
        if self._config.insecure:
            return grpc.aio.insecure_channel(self._config.endpoint, options=options)
        options.append(
            ("grpc.ssl_target_name_override", self._config.server_name or "")
        )
        credentials = grpc.ssl_channel_credentials(
            root_certificates=_read(self._config.root_certificate),
            private_key=_read(self._config.client_private_key),
            certificate_chain=_read(self._config.client_certificate),
        )
        return grpc.aio.secure_channel(
            self._config.endpoint, credentials, options=options
        )

    def _validate_handshake(self, ack: wire.HandshakeAck) -> None:
        if (
            ack.generation != self._config.protocol_generation
            or ack.connection_generation != self._generation
        ):
            raise HydraCacheError(
                ErrorCode.UNSUPPORTED,
                RetryAdvice.NEVER,
                "peer negotiated an unexpected generation",
            )
        if (
            not ack.cluster_id
            or len(ack.cluster_id) > 128
            or any(ord(ch) < 32 for ch in ack.cluster_id)
        ):
            raise HydraCacheError(
                ErrorCode.UNSUPPORTED,
                RetryAdvice.NEVER,
                "peer returned an invalid cluster identity",
            )
        requested = {int(item) for item in self._config.capabilities}
        if not requested.issubset(set(ack.accepted)):
            raise HydraCacheError(
                ErrorCode.UNSUPPORTED,
                RetryAdvice.NEVER,
                "peer omitted a required capability",
            )
        self.cluster_id = ack.cluster_id
        self.preferred_protocol_generation = (
            ack.preferred_generation or ack.generation
        )
        self.negotiated_generation_deprecated = (
            ack.negotiated_generation_deprecated
        )

    def _next_correlation(self) -> int:
        self._correlation += 1
        if self._correlation >= 2**64:
            raise HydraCacheError(
                ErrorCode.INTERNAL,
                RetryAdvice.NEVER,
                "correlation id exhausted",
            )
        return self._correlation

    def _fail_pending(self, error: HydraCacheError) -> None:
        mappings = (
            self._pending,
            self._pending_subscriptions,
            self._pending_sessions,
        )
        for mapping in mappings:
            for future in mapping.values():
                if not future.done():
                    future.set_exception(error)
            mapping.clear()


def _mutation(response: wire.InvocationResponse) -> MutationResult:
    if not response.HasField("mutation"):
        raise HydraCacheError(
            ErrorCode.INTERNAL,
            RetryAdvice.NEVER,
            "peer omitted mutation result",
        )
    return MutationResult(response.mutation.applied)


def _raise_response_error(meta: wire.ResponseMeta) -> None:
    if meta.error == wire.STABLE_ERROR_UNSPECIFIED:
        return
    codes = list(ErrorCode)
    retries = list(RetryAdvice)
    code = (
        codes[meta.error - 1]
        if 1 <= meta.error <= len(codes)
        else ErrorCode.INTERNAL
    )
    retry = (
        retries[meta.retry - 1]
        if 1 <= meta.retry <= len(retries)
        else RetryAdvice.NEVER
    )
    raise HydraCacheError(code, retry, "peer rejected the request")


def _bytes(value: bytes, name: str, minimum: int, maximum: int) -> None:
    if not isinstance(value, bytes) or not minimum <= len(value) <= maximum:
        raise ValueError(
            f"{name} byte length is outside [{minimum}, {maximum}]"
        )


def _ttl(value: int) -> None:
    if not 0 <= value <= 365 * 24 * 60 * 60 * 1000:
        raise ValueError("ttl_ms is outside the supported range")


def _read(path: Optional[str | Path]) -> bytes:
    if path is None:
        raise ValueError("missing TLS file")
    return Path(path).read_bytes()


def _unavailable(detail: str) -> HydraCacheError:
    return HydraCacheError(
        ErrorCode.UNAVAILABLE, RetryAdvice.RECONNECT_IDEMPOTENT, detail
    )


def _queue_terminal(queue: asyncio.Queue[object], item: object) -> None:
    if queue.full():
        with contextlib.suppress(asyncio.QueueEmpty):
            queue.get_nowait()
    with contextlib.suppress(asyncio.QueueFull):
        queue.put_nowait(item)
