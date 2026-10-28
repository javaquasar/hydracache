//! What a connection's blocking hook costs: the work zbus runs off the event loop, on a thread of
//! its own, through `Runtime::spawn_blocking`'s default. Everything else a connection costs on its
//! runtime is benchmarked in zruntime, against a recreation of a connection's traffic.
//!
//! The pair needs to be fresh for each operation, since a connection caches its peer's
//! credentials: it is built, untimed, inside the timed `zbus::block_on` itself, through
//! `iter_custom`, and only the lookup is timed. CodSpeed's walltime mode measures that; its CPU
//! simulation skips `iter_custom` and would ignore this id.

#[cfg(unix)]
mod unix {
    use std::{
        hint::black_box,
        os::unix::net::UnixStream,
        time::{Duration, Instant},
    };

    use criterion::{Criterion, criterion_group};
    use zbus::{Connection, connection::Builder};

    fn runtime(c: &mut Criterion) {
        let mut group = c.benchmark_group("blocking-hook");
        group.sample_size(20);
        group.bench_function("peer-credentials", |b| {
            b.iter_custom(|iters| {
                zbus::block_on(async move {
                    let mut total = Duration::ZERO;
                    for _ in 0..iters {
                        let (_server, client) = pair().await;
                        let started = Instant::now();
                        black_box(client.peer_creds().await.unwrap().clone());
                        total += started.elapsed();
                    }

                    total
                })
            });
        });
        group.finish();
    }

    /// A server and a client over a fresh socket pair, handshake included.
    async fn pair() -> (Connection, Connection) {
        let (server_end, client_end) = UnixStream::pair().unwrap();
        let server = Builder::unix_stream(server_end)
            .server(zbus::Guid::generate())
            .p2p()
            .build();
        let client = Builder::unix_stream(client_end).p2p().build();
        futures_util::try_join!(server, client).unwrap()
    }

    criterion_group!(benches, runtime);
}

#[cfg(unix)]
criterion::criterion_main!(unix::benches);

#[cfg(not(unix))]
fn main() {}
