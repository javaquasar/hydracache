//! Local shadow of the two callers' used blocking API. Not integrated/authorized IO.
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;
use zbus::{
    Error, Message, Result,
    names::{BusName, InterfaceName, MemberName},
};

#[derive(Debug, Clone)]
pub struct Connection(zbus::Connection);

impl Connection {
    pub fn from_async(connection: zbus::Connection) -> Self {
        Self(connection)
    }

    pub fn system() -> Result<Self> {
        zbus::block_on(zbus::Connection::system()).map(Self)
    }

    pub fn inner(&self) -> &zbus::Connection {
        &self.0
    }

    pub fn close(self) -> Result<()> {
        zbus::block_on(self.0.close())
    }
}

pub struct ConnectionBuilder(zbus::connection::Builder<'static>);

impl ConnectionBuilder {
    pub fn address(address: &str) -> Result<Self> {
        // Preserve the old setter's immediate conversion/refusal before any build.
        let address = zbus::Address::try_from(address)?;
        Ok(Self(zbus::connection::Builder::address(address)))
    }

    pub fn method_timeout(self, duration: Duration) -> Self {
        Self(self.0.method_timeout(duration))
    }

    pub fn build(self) -> Result<Connection> {
        zbus::block_on(self.0.build()).map(Connection)
    }
}

#[derive(Debug, Clone)]
pub struct Proxy<'a> {
    connection: Connection,
    inner: Option<zbus::Proxy<'a>>,
}

impl<'a> Proxy<'a> {
    pub fn new(
        connection: &Connection,
        destination: &'a str,
        path: &'a str,
        interface: &'a str,
    ) -> Result<Self> {
        ProxyBuilder::new(connection)
            .destination(destination)?
            .path(path)?
            .interface(interface)?
            .build()
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn call_method<'m, M, B>(&self, method: M, body: &B) -> Result<Message>
    where
        M: TryInto<MemberName<'m>>,
        M::Error: Into<Error>,
        B: Serialize + zbus::wire::DynamicType,
    {
        zbus::block_on(
            self.inner
                .as_ref()
                .expect("held proxy")
                .call_method(method, body),
        )
    }

    pub fn call<'m, M, B, R>(&self, method: M, body: &B) -> Result<R>
    where
        M: TryInto<MemberName<'m>>,
        M::Error: Into<Error>,
        B: Serialize + zbus::wire::DynamicType,
        R: for<'d> zbus::wire::DynamicDeserialize<'d>,
    {
        let reply = self.call_method(method, body)?;
        reply.body().deserialize()
    }

    pub fn get_property<T>(&self, name: &str) -> Result<T>
    where
        T: DeserializeOwned + zbus::Type,
    {
        zbus::block_on(self.inner.as_ref().expect("held proxy").get_property(name))
    }
}

impl Drop for Proxy<'_> {
    fn drop(&mut self) {
        // Match old blocking Proxy: drop the async proxy inside the runtime context.
        zbus::block_on(async {
            self.inner.take();
        });
    }
}

pub struct ProxyBuilder<'a> {
    connection: Connection,
    inner: zbus::proxy::Builder<'a, zbus::Proxy<'a>>,
}

impl<'a> ProxyBuilder<'a> {
    pub fn new(connection: &Connection) -> Self {
        Self {
            connection: connection.clone(),
            inner: zbus::proxy::Builder::new(connection.inner()),
        }
    }

    pub fn destination(self, destination: &'a str) -> Result<Self> {
        let destination = BusName::try_from(destination)?;
        Ok(Self {
            inner: self.inner.destination(destination),
            ..self
        })
    }

    pub fn path(self, path: &'a str) -> Result<Self> {
        let path = zbus::ObjectPath::try_from(path)?;
        Ok(Self {
            inner: self.inner.path(path),
            ..self
        })
    }

    pub fn interface(self, interface: &'a str) -> Result<Self> {
        let interface = InterfaceName::try_from(interface)?;
        Ok(Self {
            inner: self.inner.interface(interface),
            ..self
        })
    }

    pub fn cache_properties(self, cache: zbus::proxy::CacheProperties) -> Self {
        Self {
            inner: self.inner.cache_properties(cache),
            ..self
        }
    }

    pub fn build(self) -> Result<Proxy<'a>> {
        let proxy = zbus::block_on(self.inner.build())?;
        Ok(Proxy {
            connection: self.connection,
            inner: Some(proxy),
        })
    }
}
