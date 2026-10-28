use std::{
    ffi::OsString,
    fmt::Display,
    os::unix::{ffi::OsStrExt, process::CommandExt},
    path::PathBuf,
    process::Command,
    sync::Arc,
};

use super::encode_percents;
use crate::{
    Address, Result,
    connection::socket::BoxedSplit,
    runtime::{Runtime, process},
};

/// `unixexec:` D-Bus transport.
///
/// <https://dbus.freedesktop.org/doc/dbus-specification.html#transports-exec>
///
/// This type is only available when the `unixexec` feature is enabled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unixexec {
    path: PathBuf,
    arg0: Option<OsString>,
    args: Vec<OsString>,
}

impl Unixexec {
    /// Create a new unixexec transport with the given path and arguments.
    pub fn new(path: PathBuf, arg0: Option<OsString>, args: Vec<OsString>) -> Self {
        Self { path, arg0, args }
    }

    pub(super) fn from_options(opts: std::collections::HashMap<&str, &str>) -> crate::Result<Self> {
        let Some(path) = opts.get("path") else {
            return Err(crate::Error::Address(
                "unixexec address is missing `path`".to_owned(),
            ));
        };

        let arg0 = opts.get("argv0").map(OsString::from);

        let mut args: Vec<OsString> = Vec::new();
        let mut arg_index = 1;
        while let Some(arg) = opts.get(format!("argv{arg_index}").as_str()) {
            args.push(OsString::from(arg));
            arg_index += 1;
        }

        Ok(Self::new(PathBuf::from(path), arg0, args))
    }

    /// Binary to execute.
    ///
    /// Path of the binary to execute, either an absolute path or a binary name that is searched for
    /// in the default search path of the OS. This corresponds to the first argument of execlp().
    /// This key is mandatory.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// The executable argument.
    ///
    /// The program name to use when executing the binary. If omitted the same
    /// value as specified for path will be used. This corresponds to the
    /// second argument of execlp().
    pub fn arg0(&self) -> Option<&OsString> {
        self.arg0.as_ref()
    }

    /// Arguments.
    ///
    /// Arguments to pass to the binary.
    pub fn args(&self) -> &[OsString] {
        self.args.as_ref()
    }

    /// Runs the program and talks D-Bus over its standard input and output.
    ///
    /// The pipes are watched by `runtime`, and a task of `runtime` waits for the program to exit.
    pub(super) async fn connect(&self, address: &Address, runtime: &Runtime) -> Result<BoxedSplit> {
        process::spawn(runtime, self.command())
            .map(process::Child::into_split)
            .map_err(|e| crate::Error::Connection(Arc::new(e), Box::new(address.clone())))
    }

    /// The command this address names.
    fn command(&self) -> Command {
        let mut command = Command::new(&self.path);
        command.args(&self.args);

        if let Some(arg0) = &self.arg0 {
            command.arg0(arg0);
        }

        command
    }
}

impl Display for Unixexec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unixexec:path=")?;
        encode_percents(f, self.path.as_os_str().as_bytes())?;

        if let Some(arg0) = self.arg0.as_ref() {
            f.write_str(",argv0=")?;
            encode_percents(f, arg0.as_bytes())?;
        }

        for (index, arg) in self.args.iter().enumerate() {
            write!(f, ",argv{}=", index + 1)?;
            encode_percents(f, arg.as_bytes())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ntest::timeout;

    use crate::{
        address::{Address, transport::Transport},
        runtime::test_runtime::under_every_runtime,
    };

    #[test]
    #[timeout(15000)]
    fn connect() {
        under_every_runtime(|runtime| async move {
            let address: Address = "unixexec:path=echo,argv1=hello,argv2=world"
                .try_into()
                .unwrap();
            let Transport::Unixexec(unixexec) = address.transport() else {
                unreachable!("the address names a unixexec transport")
            };

            unixexec.connect(&address, &runtime).await.unwrap();
        });
    }

    /// A bus connection over the standard I/O of a helper process, on a runtime of the caller's.
    ///
    /// `systemd-stdio-bridge` is what plays that helper here; a machine without it has nothing
    /// to say about this and the test passes.
    #[test]
    #[timeout(15000)]
    fn a_bus_connection_over_a_helper_process() {
        use crate::{
            Error, conn::Builder, connection::Connection, fdo, runtime::test_runtime::TestRuntime,
        };

        let built = futures_lite::future::block_on(
            Builder::address("unixexec:path=systemd-stdio-bridge")
                .runtime(TestRuntime::new())
                .build(),
        );
        let connection: Connection = match built {
            Ok(connection) => connection,
            Err(Error::Connection(e, _)) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => panic!("{e}"),
        };

        // The bus answered the `Hello` the build sent, so a second one is the error that says
        // the connection is a working one.
        let called = futures_lite::future::block_on(connection.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "Hello",
            &(),
        ));

        let Err(Error::FDO(e)) = called else {
            panic!("got {called:?}");
        };
        let fdo::Error::Failed(_) = *e else {
            panic!("got {e:?}");
        };
    }
}
