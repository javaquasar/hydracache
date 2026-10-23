//! Private fixed read-only channel; no original message or execution authority.
use std::future::Future;
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use zbus::{Connection, Message};
#[path = "diagnostic_manager_channel_kernel.rs"]
mod kernel;
use kernel::{boot, clock, KernelPins, Operation, PathPins};

const SOCKET: &str = "/run/dbus/system_bus_socket";
const BUS: &str = "org.freedesktop.DBus";
const BUS_PATH: &str = "/org/freedesktop/DBus";
const MANAGER: &str = "org.freedesktop.systemd1";
#[derive(Debug, PartialEq, Eq)]
enum ChannelError {
    Io,
    Context,
    Namespace,
    Deadline,
    Reply,
    Changed,
    Refused,
}
type Result<T> = std::result::Result<T, ChannelError>;
fn io<T>(value: std::io::Result<T>) -> Result<T> {
    value.map_err(|_| ChannelError::Io)
}
fn unique(value: &str) -> bool {
    let Some(rest) = value.strip_prefix(':') else {
        return false;
    };
    rest.len() <= 64
        && rest.split('.').count() == 2
        && rest
            .split('.')
            .all(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
}
#[derive(Clone, Copy)]
enum Query {
    Owner,
    Uid,
    Pid,
}
impl Query {
    fn member(self) -> &'static str {
        match self {
            Self::Owner => "GetNameOwner",
            Self::Uid => "GetConnectionUnixUser",
            Self::Pid => "GetConnectionUnixProcessID",
        }
    }
    fn signature(self) -> &'static str {
        match self {
            Self::Owner => "s",
            _ => "u",
        }
    }
}
fn reply_shape(reply: &Message, client: &str, query: Query) -> Result<()> {
    let h = reply.header();
    if reply.message_type() != zbus::message::Type::MethodReturn
        || reply.data().bytes().len() > crate::protocol::MAX_PACKET_BYTES
        || reply.primary_header().flags().bits() & !(zbus::message::Flags::NoReplyExpected as u8)
            != 0
        || h.sender().map(|v| v.as_str()) != Some(BUS)
        || h.destination().map(|v| v.as_str()) != Some(client)
        || h.reply_serial().is_none()
        || h.error_name().is_some()
        || h.path().is_some()
        || h.interface().is_some()
        || h.member().is_some()
        || !reply.data().fds().is_empty()
        || h.unix_fds().unwrap_or(0) != 0
        || reply.body().signature().to_string() != query.signature()
    {
        return Err(ChannelError::Reply);
    }
    Ok(())
}
async fn query<R: for<'de> zbus::zvariant::DynamicDeserialize<'de>>(
    connection: &Connection,
    client: &str,
    kind: Query,
    argument: &str,
) -> Result<R> {
    // zbus correlates the original reply serial on this same connection.
    // No caller can select a member, destination, path, interface or payload.
    let response = connection
        .call_method(Some(BUS), BUS_PATH, Some(BUS), kind.member(), &(argument,))
        .await;
    #[cfg(test)]
    tests::trace_query(&response, client, kind);
    let reply = response.map_err(|_| ChannelError::Reply)?;
    reply_shape(&reply, client, kind)?;
    reply.body().deserialize().map_err(|_| ChannelError::Reply)
}
async fn owner(connection: &Connection, client: &str) -> Result<String> {
    let name: String = query(connection, client, Query::Owner, MANAGER).await?;
    if !unique(&name) || name == client {
        return Err(ChannelError::Reply);
    }
    let uid: u32 = query(connection, client, Query::Uid, &name).await?;
    let pid: u32 = query(connection, client, Query::Pid, &name).await?;
    if uid != 0 || pid != 1 {
        return Err(ChannelError::Context);
    }
    Ok(name)
}

// No Clone, serde, snapshot constructor, connection/name/FD getter or sender.
struct FixedManagerChannel {
    connection: Connection,
    paths: PathPins,
    kernel: KernelPins,
    boot: String,
    manager: String,
    client: String,
    guid: String,
    refused: bool,
    runtime: Runtime,
}
impl std::fmt::Debug for FixedManagerChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixedManagerChannel")
            .field("refused", &self.refused)
            .finish_non_exhaustive()
    }
}
impl FixedManagerChannel {
    fn open_fixed() -> Result<Self> {
        let operation = Operation::new()?;
        let runtime = io(Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build())?;
        let paths = PathPins::fixed()?;
        let initial_boot = boot()?;
        let (connection, kernel, client, manager, guid) =
            runtime.block_on(operation.bound(async {
                let stream = io(tokio::net::UnixStream::connect(SOCKET).await)?;
                let kernel = KernelPins::capture(&stream)?;
                paths.inspect()?;
                if boot()? != initial_boot {
                    return Err(ChannelError::Changed);
                }
                let connection = zbus::connection::Builder::unix_stream(stream)
                    .auth_mechanism(zbus::AuthMechanism::External)
                    .max_queued(16)
                    .method_timeout(Duration::from_millis(250))
                    .build()
                    .await
                    .map_err(|_| ChannelError::Io)?;
                if !connection.is_bus() || connection.is_closed() {
                    return Err(ChannelError::Context);
                }
                let client = connection
                    .unique_name()
                    .map(|v| v.as_str().to_owned())
                    .ok_or(ChannelError::Context)?;
                if !unique(&client) {
                    return Err(ChannelError::Context);
                }
                let guid = connection.server_guid().to_string();
                let manager = owner(&connection, &client).await?;
                if owner(&connection, &client).await? != manager {
                    return Err(ChannelError::Changed);
                }
                kernel.inspect()?;
                paths.inspect()?;
                if boot()? != initial_boot {
                    return Err(ChannelError::Changed);
                }
                Ok((connection, kernel, client, manager, guid))
            }))?;
        let channel = Self {
            connection,
            paths,
            kernel,
            boot: initial_boot,
            manager,
            client,
            guid,
            refused: false,
            runtime,
        };
        channel.inspect()?;
        if clock()? >= operation.deadline {
            return Err(ChannelError::Deadline);
        }
        Ok(channel)
    }
    fn inspect(&self) -> Result<()> {
        if self.refused {
            return Err(ChannelError::Refused);
        }
        if self.connection.is_closed()
            || !self.connection.is_bus()
            || self.connection.unique_name().map(|v| v.as_str()) != Some(self.client.as_str())
            || self.connection.server_guid().as_str() != self.guid
            || boot()? != self.boot
        {
            return Err(ChannelError::Changed);
        }
        self.kernel.inspect()?;
        self.paths.inspect()
    }
    fn revalidate(&mut self) -> Result<()> {
        if self.refused {
            return Err(ChannelError::Refused);
        }
        let result = (|| {
            let operation = Operation::new()?;
            self.inspect()?;
            self.runtime.block_on(operation.bound(async {
                if owner(&self.connection, &self.client).await? != self.manager {
                    return Err(ChannelError::Changed);
                }
                self.inspect()?;
                Ok(())
            }))
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

impl Operation {
    async fn bound<T>(&self, future: impl Future<Output = Result<T>>) -> Result<T> {
        if clock()? >= self.deadline {
            return Err(ChannelError::Deadline);
        }
        let alarm = io(tokio::io::unix::AsyncFd::new(io(self.file.try_clone())?))?;
        tokio::select! {
            biased;
            readiness=alarm.readable() => { let _guard=readiness.map_err(|_|ChannelError::Io)?; Err(ChannelError::Deadline) }
            result=future => {
                if clock()? >= self.deadline { Err(ChannelError::Deadline) } else { result }
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_manager_channel_tests.rs"]
mod tests;

#[path = "diagnostic_original_manager_join.rs"]
mod original_join;

#[path = "diagnostic_manager_sandbox_query.rs"]
mod sandbox_query;
