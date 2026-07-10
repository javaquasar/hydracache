use std::sync::Arc;

use hydracache_redis_compat::RedisRespServer;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::admin_http::SharedServerRuntime;

/// TCP accept-loop failures for the optional Redis RESP listener.
#[derive(Debug, Error)]
pub enum RedisTcpError {
    /// Accepting a TCP connection failed.
    #[error("redis tcp accept error: {0}")]
    Accept(#[from] std::io::Error),
}

/// Serve the optional Redis RESP listener until shutdown is requested.
pub async fn serve_redis_listener(
    listener: TcpListener,
    server: Arc<RedisRespServer>,
    runtime: SharedServerRuntime,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), RedisTcpError> {
    loop {
        if *shutdown.borrow() {
            return Ok(());
        }
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return Ok(());
                }
            }
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                if !runtime
                    .lock()
                    .expect("server runtime mutex")
                    .begin_redis_connection()
                {
                    continue;
                }
                let guard = RedisConnectionGuard::new(Arc::clone(&runtime));
                let server = Arc::clone(&server);
                tokio::spawn(async move {
                    let _guard = guard;
                    let _ = server.serve_connection(stream).await;
                });
            }
        }
    }
}

struct RedisConnectionGuard {
    runtime: SharedServerRuntime,
}

impl RedisConnectionGuard {
    fn new(runtime: SharedServerRuntime) -> Self {
        Self { runtime }
    }
}

impl Drop for RedisConnectionGuard {
    fn drop(&mut self) {
        self.runtime
            .lock()
            .expect("server runtime mutex")
            .finish_redis_connection();
    }
}
