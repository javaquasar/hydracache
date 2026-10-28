//! Async child processes on a [`Runtime`].
//!
//! The API has the shape of [`std::process`], like `smol::process`. A [`Command`] builds the
//! process to spawn. The [`Child`] it spawns holds the pipes to the process, [`ChildStdin`],
//! [`ChildStdout`] and [`ChildStderr`], and has futures that wait for the process to exit. Nothing
//! here blocks the thread: when an operation has to wait, the thread can run other tasks.
//!
//! The pipes implement the `AsyncWrite` and `AsyncRead` traits of [`futures-io`], so the extension
//! traits of [`futures`] work on them.
//!
//! # The runtime
//!
//! A child is spawned on a runtime, which [`Command::spawn`], [`Command::status`] and
//! [`Command::output`] take as an argument.
//!
//! On unix, the runtime's reactor watches the pipes to the child. Reads and writes make progress
//! while a thread is running [`Runtime::block_on`] on that runtime. They also make progress while
//! the helper thread runs the runtime, as it does for one from `SharedRuntime::current`. On Linux
//! (with pidfd support), on Apple's platforms and on the BSDs, the reactor also watches for the
//! process to exit, so a wait for the exit makes progress in the same way.
//!
//! On Windows, the reactor cannot watch a pipe. Each read from and write to a pipe runs as blocking
//! work on a thread of the pool of [`unblock()`](crate::unblock()), so it makes progress even
//! while no thread is running `block_on`. The same holds for a wait for the exit on any platform
//! where the reactor cannot watch for it, which includes Windows. That wait runs on a pool of its
//! own: see [waiting for a child](self#waiting-for-a-child).
//!
//! `Child` and the pipes are generic over the runtime's flavour, on every platform. A child spawned
//! on a [`LocalRuntime`] is a `Child<Local>` (`Local` is the default). It stays on the thread that
//! created it, with its pipes. A child spawned on a [`SharedRuntime`] is a `Child<Shared>`, which
//! can be sent to and used from any thread.
//!
//! # Waiting for a child
//!
//! [`Child::status`] waits for the process to exit and resolves to its exit status.
//! [`Child::output`] does the same after it has read the output pipes.
//!
//! On unix, a process that has exited stays in the process table as a zombie, and keeps its
//! process ID, until its status is collected. [`Child::status`] collects the status when it
//! resolves, and so does [`Child::try_status`], which never waits.
//!
//! If the runtime can watch for the exit, the wait holds no thread:
//!
//! - On Linux, the runtime watches a pidfd, a descriptor of the process that becomes readable when
//!   the process exits.
//! - On Apple's platforms and the BSDs, it watches a kqueue that belongs to the child alone, with a
//!   filter on the process. The descriptor of the queue becomes readable when the process exits.
//!
//! The reactor watches the descriptor like a pipe, from the spawn until it reports the exit.
//! Dropping the future of a wait leaves nothing running.
//!
//! On Apple's platforms and the BSDs, there is no other way to wait. If the kqueue cannot be
//! created, for example because the process is out of descriptors, the spawn fails with that
//! error, as it does if a pipe cannot be created. On Linux, a process that another process traces
//! cannot be collected after it exits until its tracer lets it go. A wait that finds the process in
//! that state continues on the pool described next.
//!
//! The runtime cannot watch for the exit on Android, on Windows, on other unix systems, and on a
//! Linux without pidfd support (before 5.3, or in a sandbox that rejects the call). There, the wait
//! runs as blocking work on a thread of a pool kept for waits for children, and holds that thread
//! until the process exits. The first wait that finds the process still running starts the work.
//! Spawning does not, so a child that nobody waits for holds no thread. Dropping the future of a
//! wait gives up the wait but not the thread. The next wait for the same child takes it up where it
//! left off, so abandoned waits do not pile up threads.
//!
//! The pool is separate from the one behind [`unblock()`](crate::unblock()). Otherwise a program
//! that waits for many long-running children at once would fill that pool, and all its other
//! blocking work would wait behind them. A [dropped child](self#dropping-a-child) is collected on
//! the same pool.
//!
//! # Dropping a child
//!
//! Dropping a [`Child`] closes the pipes it still holds. It leaves the process running, as
//! dropping a std `Child` does. [`Command::kill_on_drop`] kills the process instead.
//!
//! On unix, a process that exits with nobody to collect its status stays a zombie, and std's
//! `Child` leaves it so. By default, a `Child` that is dropped while its process is running hands
//! the process to a thread of the pool for waits for children. That thread waits for the process
//! to exit and collects its status. [`Command::reap_on_drop`] turns this off.
//!
//! Each such thread is held until its process exits, so dropping many long-running children holds
//! as many threads. The pool has room for 500 threads. Beyond that, a dropped child waits for a
//! free thread, and stays a zombie if it exits before one is free. Killing the processes, waiting
//! for them, or turning `reap_on_drop` off keeps threads from being held for long.
//!
//! The threads are named `zruntime child wait`. The pool is separate from the one behind
//! [`unblock()`](crate::unblock()), which all other blocking work shares. However many children
//! are dropped, and however long they run, that work never waits behind them.
//!
//! Windows leaves no zombie, so there is nothing to collect there, and `reap_on_drop` does
//! nothing.
//!
//! # Example
//!
//! Runs a command on a local runtime and checks its output:
//!
//! ```
//! # #[cfg(unix)]
//! # fn main() -> std::io::Result<()> {
//! use zruntime::{LocalRuntime, process::Command};
//!
//! let runtime = LocalRuntime::new()?;
//!
//! let output = runtime.block_on(Command::new("echo").arg("hello").output(&runtime))?;
//!
//! assert!(output.status.success());
//! assert_eq!(output.stdout, b"hello\n");
//! # Ok(())
//! # }
//! # #[cfg(not(unix))]
//! # fn main() {}
//! ```
//!
//! [`futures-io`]: https://docs.rs/futures-io
//! [`futures`]: https://docs.rs/futures
//! [`LocalRuntime`]: crate::LocalRuntime
//! [`Runtime`]: crate::Runtime
//! [`Runtime::block_on`]: crate::Runtime::block_on
//! [`SharedRuntime`]: crate::SharedRuntime

mod exit;
mod stdio;

use std::{
    ffi::OsStr,
    fmt,
    future::{Future, poll_fn},
    io,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use futures_io::AsyncRead;

use self::exit::Exit;
pub use self::stdio::{ChildStderr, ChildStdin, ChildStdout};
use crate::{
    Local, Mode, Runtime,
    unblock::{IDLE_TIMEOUT, MAX_THREADS, pool::Pool},
};
pub use std::process::{ExitStatus, Output, Stdio};

/// A builder for a process to spawn, like [`std::process::Command`].
///
/// The methods that spawn the process take the runtime to spawn it on. Configure the command as
/// with std: its arguments, environment, working directory and the standard streams of the
/// process. A command from `Command::new` starts with the environment and working directory of
/// this process.
///
/// [`as_std`](Command::as_std) gives the std command inside, for its getters of the program, the
/// arguments, the environment and the working directory.
/// [`as_std_mut`](Command::as_std_mut) gives it for the extension traits of the platform, such as
/// std's `CommandExt` on unix.
///
/// What happens to the process when its [`Child`] is dropped is set on this builder, not on the
/// std command. [`kill_on_drop`](Command::kill_on_drop) and
/// [`reap_on_drop`](Command::reap_on_drop) set it, and
/// [`get_kill_on_drop`](Command::get_kill_on_drop) and
/// [`get_reap_on_drop`](Command::get_reap_on_drop) read it.
///
/// A command can run more than once. A standard stream that was not configured gets the default of
/// the method that runs the command, afresh on each run. [`spawn`](Command::spawn) and
/// [`status`](Command::status) inherit it from this process. [`output`](Command::output) connects
/// the standard input to nothing and the other two streams to pipes, which it reads to their ends.
///
/// `spawn`, `status` and `output` start the process when they are called, before any future is
/// polled, and it runs from then on. The futures of `status` and `output` only wait for it. If the
/// spawn fails, `spawn` returns the error, and the other two resolve to it.
///
/// # Example
///
/// Runs a command and checks its exit status. Its output goes to this process's own:
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> std::io::Result<()> {
/// use zruntime::{LocalRuntime, process::Command};
///
/// let runtime = LocalRuntime::new()?;
///
/// let status = runtime.block_on(Command::new("true").status(&runtime))?;
///
/// assert!(status.success());
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
pub struct Command {
    inner: std::process::Command,
    // Whether each stream was set through this builder: one that was not gets the default of
    // whichever of `spawn`, `status` and `output` runs the command, every time, so that a command
    // run by one of them and then by another does not carry the first one's defaults along.
    stdin: bool,
    stdout: bool,
    stderr: bool,
    kill_on_drop: bool,
    reap_on_drop: bool,
}

impl Command {
    /// Creates a command that runs `program`.
    ///
    /// The command has no arguments, inherits the environment and working directory of this
    /// process, and has no standard streams configured. If `program` has no path in it, it is
    /// looked up on the `PATH`, as for std's [`Command::new`](std::process::Command::new).
    pub fn new<S>(program: S) -> Self
    where
        S: AsRef<OsStr>,
    {
        Self::from(std::process::Command::new(program))
    }

    /// Adds an argument to pass to the program.
    pub fn arg<S>(&mut self, arg: S) -> &mut Self
    where
        S: AsRef<OsStr>,
    {
        self.inner.arg(arg);
        self
    }

    /// Adds arguments to pass to the program.
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }

    /// Sets an environment variable of the process.
    ///
    /// On Windows, names of environment variables are case-insensitive but case-preserving. On
    /// other platforms, they are case-sensitive.
    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.env(key, value);
        self
    }

    /// Sets environment variables of the process.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.envs(vars);
        self
    }

    /// Removes an environment variable from the environment of the process, whether it is inherited
    /// from this process or was set on the command.
    pub fn env_remove<K>(&mut self, key: K) -> &mut Self
    where
        K: AsRef<OsStr>,
    {
        self.inner.env_remove(key);
        self
    }

    /// Clears the environment of the process, so that it inherits none of this process's variables.
    pub fn env_clear(&mut self) -> &mut Self {
        self.inner.env_clear();
        self
    }

    /// Sets the working directory of the process.
    pub fn current_dir<P>(&mut self, dir: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.inner.current_dir(dir);
        self
    }

    /// Configures what the standard input of the process is connected to.
    ///
    /// With `Stdio::piped()`, the child's [`stdin`](Child::stdin) is the pipe to the process.
    pub fn stdin<T>(&mut self, cfg: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.stdin = true;
        self.inner.stdin(cfg);
        self
    }

    /// Configures what the standard output of the process is connected to.
    ///
    /// With `Stdio::piped()`, the child's [`stdout`](Child::stdout) is the pipe from the process.
    pub fn stdout<T>(&mut self, cfg: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.stdout = true;
        self.inner.stdout(cfg);
        self
    }

    /// Configures what the standard error of the process is connected to.
    ///
    /// With `Stdio::piped()`, the child's [`stderr`](Child::stderr) is the pipe from the process.
    pub fn stderr<T>(&mut self, cfg: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.stderr = true;
        self.inner.stderr(cfg);
        self
    }

    /// Sets whether the process is killed when its [`Child`] is dropped.
    ///
    /// The default is `false`: dropping the `Child` leaves the process running. See the
    /// [module documentation](self#dropping-a-child). The status of a killed process is collected
    /// like that of any other dropped process, unless [`reap_on_drop`](Command::reap_on_drop) is
    /// off.
    pub fn kill_on_drop(&mut self, kill_on_drop: bool) -> &mut Self {
        self.kill_on_drop = kill_on_drop;
        self
    }

    /// Sets whether the status of the process is collected when its [`Child`] is dropped while the
    /// process is still running.
    ///
    /// The default is `true`. On unix, a process that exits with nobody to collect its status stays
    /// a zombie and keeps its process ID. With this on, dropping a running `Child` hands the
    /// process to a thread of a pool for waits for children. The thread waits for the process to
    /// exit and collects its status, and is held until then. Turning this off spares the thread but
    /// leaves the zombie: see the [module documentation](self#dropping-a-child).
    ///
    /// This does nothing on Windows, where an exited process leaves nothing to collect.
    pub fn reap_on_drop(&mut self, reap_on_drop: bool) -> &mut Self {
        self.reap_on_drop = reap_on_drop;
        self
    }

    /// Whether the process is killed when its [`Child`] is dropped.
    ///
    /// The default is `false`. [`kill_on_drop`](Command::kill_on_drop) sets it.
    pub fn get_kill_on_drop(&self) -> bool {
        self.kill_on_drop
    }

    /// Whether the status of the process is collected when its [`Child`] is dropped while the
    /// process is still running.
    ///
    /// The default is `true`. [`reap_on_drop`](Command::reap_on_drop) sets it. It makes no
    /// difference on Windows, where an exited process leaves nothing to collect.
    pub fn get_reap_on_drop(&self) -> bool {
        self.reap_on_drop
    }

    /// The std command inside this one, to read the program, the arguments, the environment and the
    /// working directory.
    ///
    /// The std command knows nothing about what happens when the [`Child`] is dropped. Read that
    /// with [`get_kill_on_drop`](Command::get_kill_on_drop) and
    /// [`get_reap_on_drop`](Command::get_reap_on_drop).
    pub fn as_std(&self) -> &std::process::Command {
        &self.inner
    }

    /// The std command inside this one, to configure with what only it has.
    ///
    /// That is the std extension traits for the platform, such as `uid`, `pre_exec` and
    /// `process_group` on unix, or `creation_flags` on Windows.
    ///
    /// Do not configure the standard streams through it. This builder does not know about a stream
    /// set there, so the method that runs the command overrides it with its default. Use
    /// [`stdin`](Command::stdin), [`stdout`](Command::stdout) and [`stderr`](Command::stderr)
    /// instead.
    pub fn as_std_mut(&mut self) -> &mut std::process::Command {
        &mut self.inner
    }

    /// Spawns the process on `runtime` and returns its [`Child`].
    ///
    /// The process is running when this returns. A standard stream that the command was not told
    /// about is inherited from this process. A stream set to `Stdio::piped()` is the matching pipe
    /// field of the `Child`. On unix, `runtime` watches the pipes. On Windows, they run on a thread
    /// of the pool of [`unblock()`](crate::unblock()).
    ///
    /// # Errors
    ///
    /// Fails if the process cannot be spawned, for the reasons given by std's
    /// [`spawn`](std::process::Command::spawn), for example a program that does not exist or may
    /// not run.
    ///
    /// Also fails if `runtime` cannot start watching the pipes, or the descriptor for the exit of
    /// the process if the runtime watches for the exit. The poller of the system may refuse to
    /// watch any of them. On Apple's platforms and the BSDs, the system may also refuse to create
    /// that descriptor, for example if the process is out of descriptors, because there is no other
    /// way to wait there. The process is then killed and its status is collected, so it is not left
    /// running with nobody holding it.
    pub fn spawn<M>(&mut self, runtime: &Runtime<M>) -> io::Result<Child<M>>
    where
        M: Mode,
    {
        self.start(runtime, Stdio::inherit, Stdio::inherit, Stdio::inherit)
    }

    /// Spawns the process on `runtime`, waits for it to exit and resolves to its exit status.
    ///
    /// A standard stream that the command was not told about is inherited from this process, as for
    /// [`spawn`](Command::spawn). The pipe to the standard input, if the command asked for one, is
    /// closed before the wait, as [`Child::status`] does. A pipe from the process that nothing
    /// reads may fill and keep the process from ever exiting. Use [`output`](Command::output) for
    /// a process whose output you want.
    ///
    /// The process is spawned by this call, before the future is polled, and runs whether or not
    /// the future is polled. Dropping the future gives up the wait and drops the child. The process
    /// keeps running unless the command set [`kill_on_drop`](Command::kill_on_drop).
    ///
    /// # Errors
    ///
    /// The future resolves to the error if the spawn fails, for the reasons given for
    /// [`spawn`](Command::spawn), or if the wait fails, for the reasons given for
    /// [`Child::status`].
    pub fn status<M>(
        &mut self,
        runtime: &Runtime<M>,
    ) -> impl Future<Output = io::Result<ExitStatus>> + use<M>
    where
        M: Mode,
    {
        let child = self.start(runtime, Stdio::inherit, Stdio::inherit, Stdio::inherit);

        async move {
            let mut child = child?;

            child.status().await
        }
    }

    /// Spawns the process on `runtime` and resolves to its output and exit status once it exits.
    ///
    /// The output holds what the process wrote to its standard output and its standard error. A
    /// standard stream that the command was not told about is connected to nothing for the standard
    /// input, and to a pipe for each of the standard output and error, which are read to their
    /// ends. A stream that the command was told about is as it was told, and a stream that is not
    /// piped is not captured: the output holds nothing of it. See [`Child::output`] for how the
    /// pipes are read.
    ///
    /// The process is spawned by this call, before the future is polled, and runs whether or not
    /// the future is polled. Dropping the future gives up the wait and drops the child. The process
    /// keeps running unless the command set [`kill_on_drop`](Command::kill_on_drop).
    ///
    /// # Errors
    ///
    /// The future resolves to the error if the spawn fails, for the reasons given for
    /// [`spawn`](Command::spawn), or if reading the pipes or the wait fails, for the reasons given
    /// for [`Child::output`].
    pub fn output<M>(
        &mut self,
        runtime: &Runtime<M>,
    ) -> impl Future<Output = io::Result<Output>> + use<M>
    where
        M: Mode,
    {
        let child = self.start(runtime, Stdio::null, Stdio::piped, Stdio::piped);

        async move { child?.output().await }
    }

    /// Spawns the process on `runtime`, with each standard stream that this builder was not told
    /// about connected to what the function for it makes.
    fn start<M>(
        &mut self,
        runtime: &Runtime<M>,
        stdin: fn() -> Stdio,
        stdout: fn() -> Stdio,
        stderr: fn() -> Stdio,
    ) -> io::Result<Child<M>>
    where
        M: Mode,
    {
        // Set every time rather than once, as the std command keeps what a run before this one
        // gave it.
        if !self.stdin {
            self.inner.stdin(stdin());
        }
        if !self.stdout {
            self.inner.stdout(stdout());
        }
        if !self.stderr {
            self.inner.stderr(stderr());
        }

        let child = self.inner.spawn()?;

        Child::new(runtime, child, self.kill_on_drop, self.reap_on_drop)
    }
}

/// Wraps a std command.
///
/// The standard streams configured on `command` count as not configured, so the method that runs
/// the command overrides them with its defaults. As with [`Command::new`], the process is not
/// killed when its [`Child`] is dropped, and its status is collected where the platform needs
/// that.
impl From<std::process::Command> for Command {
    fn from(command: std::process::Command) -> Self {
        Self {
            inner: command,
            stdin: false,
            stdout: false,
            stderr: false,
            kill_on_drop: false,
            reap_on_drop: true,
        }
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

/// A spawned child process, with the pipes to it that its command asked for.
///
/// [`Command::spawn`] creates it on a runtime. A `Child<Local>` (`Local` is the default) stays on
/// the thread that created it, and a `Child<Shared>` can be sent to and used from any thread. See
/// the [module documentation](self) for what drives it.
///
/// The pipes are public fields. Each is `Some` if the command asked for it with `Stdio::piped()`,
/// and `None` otherwise. To move a pipe elsewhere, take it out, as in `child.stdout.take()` or
/// `let Child { stdout, .. } = child;`. `Child` has no `Drop` implementation, so this works for
/// every field.
///
/// Dropping a `Child` leaves the process running unless the command said otherwise: see the
/// [module documentation](self#dropping-a-child).
///
/// # Example
///
/// Reads the output of a child as it is written, then waits for its exit status:
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> std::io::Result<()> {
/// use futures::AsyncReadExt;
/// use zruntime::{
///     LocalRuntime,
///     process::{Command, Stdio},
/// };
///
/// let runtime = LocalRuntime::new()?;
///
/// runtime.block_on(async {
///     let mut child = Command::new("echo")
///         .arg("hello")
///         .stdout(Stdio::piped())
///         .spawn(&runtime)?;
///
///     let mut greeting = String::new();
///     let mut stdout = child.stdout.take().expect("stdout is piped");
///     stdout.read_to_string(&mut greeting).await?;
///
///     assert_eq!(greeting, "hello\n");
///     assert!(child.status().await?.success());
///     # Ok::<_, std::io::Error>(())
/// })?;
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
pub struct Child<M = Local>
where
    M: Mode,
{
    /// The pipe to the standard input of the process, if its command asked for one.
    ///
    /// Dropping it closes the pipe, and a child that reads its input to the end sees the end then.
    /// [`status`](Child::status) and [`output`](Child::output) drop it themselves.
    pub stdin: Option<ChildStdin<M>>,
    /// The pipe from the standard output of the process, if its command asked for one.
    pub stdout: Option<ChildStdout<M>>,
    /// The pipe from the standard error of the process, if its command asked for one.
    pub stderr: Option<ChildStderr<M>>,
    // Declared after the pipes, so that they close before the guard kills the process or hands it
    // on.
    guard: Guard,
    exit: Exit<M>,
}

impl<M> Child<M>
where
    M: Mode,
{
    /// The process ID of the child.
    ///
    /// The ID refers to this process until it has exited and its status has been collected, by
    /// [`status`](Child::status) or [`try_status`](Child::try_status). After that, the system may
    /// give it to another process.
    pub fn id(&self) -> u32 {
        self.guard.get().id()
    }

    /// Forces the process to exit, without waiting for it to do so.
    ///
    /// This sends `SIGKILL` on unix and calls `TerminateProcess` on Windows. To wait until the
    /// process is gone, call [`status`](Child::status) afterwards. As with std's
    /// [`kill`](std::process::Child::kill), killing a process whose status has already been
    /// collected is not an error.
    ///
    /// # Errors
    ///
    /// Fails if the OS cannot kill the process.
    pub fn kill(&mut self) -> io::Result<()> {
        self.guard.get_mut().kill()
    }

    /// Checks whether the process has exited, without waiting.
    ///
    /// Returns the exit status if it has, and `None` if it is still running. Unlike
    /// [`status`](Child::status), this leaves the pipe to the standard input open. When the process
    /// has exited, this collects its status, and every later call returns the same status.
    ///
    /// # Errors
    ///
    /// Fails if the OS cannot report the status of the process.
    pub fn try_status(&mut self) -> io::Result<Option<ExitStatus>> {
        self.guard.get_mut().try_wait()
    }

    /// Waits for the process to exit and resolves to its exit status.
    ///
    /// The pipe to the standard input is dropped first, as std's `wait` does. Otherwise a process
    /// that reads its input to the end would wait for the pipe to close, while this waits for the
    /// process to exit. The pipes from the process stay open. A process that fills a pipe that
    /// nothing reads blocks, so read them while waiting, or use [`output`](Child::output), which
    /// does.
    ///
    /// If the process has already exited, or its status was collected earlier, this resolves at
    /// once, with the same status each time. See the
    /// [module documentation](self#waiting-for-a-child) for how the wait works.
    ///
    /// # Errors
    ///
    /// Fails if the OS cannot report the status of the process, or the runtime cannot wait for it.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future loses nothing. The next call takes up the wait, and finds a process that
    /// exited in the meantime as it is.
    pub async fn status(&mut self) -> io::Result<ExitStatus> {
        drop(self.stdin.take());

        loop {
            if let Some(status) = self.guard.get_mut().try_wait()? {
                return Ok(status);
            }
            // The wait is for the exit alone, and leaves the status to be collected by the
            // `try_wait` above, so a wait that ends is followed by a look at the status, and by
            // another wait only in the unlikely event that it is not there.
            self.exit.wait(self.guard.get()).await?;
        }
    }

    /// Reads everything the process writes to its standard output and error, then waits for it to
    /// exit.
    ///
    /// The pipe to the standard input is dropped first, so that a process that reads its input to
    /// the end does not wait for this to close it. Then the pipes from the process, those it has,
    /// are read to their ends, both at the same time. Reading them one after the other could
    /// deadlock: a process that fills one pipe while this reads the other would never write the
    /// rest. A stream that is not piped is not captured, and the output holds nothing of it.
    /// Finally this waits for the process, as [`status`](Child::status) does.
    ///
    /// Only the pipes of this child are read. To run a command for its output, use
    /// [`Command::output`], which pipes the right ones.
    ///
    /// # Errors
    ///
    /// Fails if reading a pipe fails, or if waiting for the process fails. If reading fails, this
    /// does not wait for the process, and the child is dropped.
    ///
    /// # Cancel safety
    ///
    /// Not cancel safe. This takes the child by value, so dropping the future drops the child and
    /// discards what was read. See [Dropping a child](self#dropping-a-child) for what that does to
    /// the process.
    pub async fn output(mut self) -> io::Result<Output> {
        drop(self.stdin.take());

        let mut stdout = Capture::new(self.stdout.take());
        let mut stderr = Capture::new(self.stderr.take());
        poll_fn(|cx| {
            // Both are polled each time, so that each is woken for its own pipe.
            let stdout = stdout.poll_end(cx);
            let stderr = stderr.poll_end(cx);

            match (stdout, stderr) {
                (Poll::Ready(Err(error)), _) | (_, Poll::Ready(Err(error))) => {
                    Poll::Ready(Err(error))
                }
                (Poll::Ready(Ok(())), Poll::Ready(Ok(()))) => Poll::Ready(Ok(())),
                _ => Poll::Pending,
            }
        })
        .await?;

        let status = self.status().await?;

        Ok(Output {
            status,
            stdout: stdout.data,
            stderr: stderr.data,
        })
    }

    /// A child for `child`, the process just spawned on `runtime`.
    ///
    /// The pipes of `child`, and where the runtime watches for the exit of its process, the
    /// descriptor that tells of it, are put under the watch of `runtime`, and what the child is to
    /// do when it is dropped is set to `kill_on_drop` and `reap_on_drop`.
    fn new(
        runtime: &Runtime<M>,
        child: std::process::Child,
        kill_on_drop: bool,
        reap_on_drop: bool,
    ) -> io::Result<Self> {
        // The process is killed, and its status collected, should anything below fail, so that a
        // spawn that fails leaves no process behind, with nobody holding it. What the command
        // asked for applies once the child is whole.
        let mut guard = Guard::new(child, true, true);
        let child = guard.get_mut();
        let stdin = child
            .stdin
            .take()
            .map(|pipe| ChildStdin::new(runtime, pipe))
            .transpose()?;
        let stdout = child
            .stdout
            .take()
            .map(|pipe| ChildStdout::new(runtime, pipe))
            .transpose()?;
        let stderr = child
            .stderr
            .take()
            .map(|pipe| ChildStderr::new(runtime, pipe))
            .transpose()?;
        let exit = Exit::new(runtime, child)?;
        guard.kill_on_drop = kill_on_drop;
        guard.reap_on_drop = reap_on_drop;

        Ok(Self {
            stdin,
            stdout,
            stderr,
            guard,
            exit,
        })
    }
}

impl<M> fmt::Debug for Child<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Child")
            .field("id", &self.id())
            .field("stdin", &self.stdin)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .finish_non_exhaustive()
    }
}

/// The std child, and what is to be done with its process when the `Child` is dropped.
///
/// A guard of its own rather than a `Drop` of `Child`, which would keep the pipes from being moved
/// out of the `Child` by the code that holds it.
struct Guard {
    /// Always there, but while the guard is dropped, which takes it out to hand it on.
    child: Option<std::process::Child>,
    /// Whether the process is killed.
    kill_on_drop: bool,
    /// Whether a process that is still running is waited for, so that its status is collected
    /// once it has exited, where the platform has a status to collect.
    reap_on_drop: bool,
}

impl Guard {
    /// A guard of `child`, which kills its process on drop if `kill_on_drop` says so, and hands it
    /// on to be collected if `reap_on_drop` does.
    fn new(child: std::process::Child, kill_on_drop: bool, reap_on_drop: bool) -> Self {
        Self {
            child: Some(child),
            kill_on_drop,
            reap_on_drop,
        }
    }

    /// The std child.
    fn get(&self) -> &std::process::Child {
        let Some(child) = &self.child else {
            unreachable!("the child is there until the guard is dropped");
        };

        child
    }

    /// The std child, to kill it, or to look at whether it has exited.
    fn get_mut(&mut self) -> &mut std::process::Child {
        let Some(child) = &mut self.child else {
            unreachable!("the child is there until the guard is dropped");
        };

        child
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };

        if self.kill_on_drop {
            // It may have exited already, or be one that cannot be killed, and either way there is
            // nobody left to tell.
            let _ = child.kill();
        }
        if self.reap_on_drop {
            reap(child);
        }
    }
}

/// Sees to it that the status of `child`'s process is collected, once it has exited.
///
/// A process that has exited already is collected by the look at it. One that is still running is
/// handed to a thread of [`WAITS`], which waits for it to exit, and nothing awaits the outcome: the
/// thread collects the status and that is all.
#[cfg(unix)]
fn reap(mut child: std::process::Child) {
    if matches!(child.try_wait(), Ok(None)) {
        Pool::submit(
            &WAITS,
            Box::new(move || {
                let _ = child.wait();
            }),
        );
    }
}

/// A process that exits leaves no status on Windows to be collected, so there is nothing to do.
#[cfg(windows)]
fn reap(child: std::process::Child) {
    drop(child);
}

/// The pool that the waits for children run on, apart from that of
/// [`unblock()`](crate::unblock()): the waits that reap a child that is dropped while its process
/// runs, and, where the runtime cannot watch for the exit of a child, the waits for it.
///
/// A wait for a child holds its thread for as long as the child runs, which may be as long as the
/// program does. Such waits are not the work the pool of `unblock()` is for, which ends of its own
/// accord, and a program that lets go of enough children, or waits for enough of them at once,
/// would hold every thread of that pool with them, for every other piece of blocking work to wait
/// behind. So they have a pool of their own, whose threads hold nothing else: it holds a thread for
/// each wait that runs, up to [`MAX_THREADS`], and past that a wait waits its turn behind the other
/// waits for children, never behind other blocking work. Its threads are named
/// `zruntime child wait`. A child that is dropped after a wait for its exit was given up on holds
/// two of them until its process exits: that of the wait, which runs on, and that of the reap.
static WAITS: Pool = Pool::new("zruntime child wait", MAX_THREADS, IDLE_TIMEOUT);

/// What [`Child::output`] reads one of the child's pipes into.
struct Capture<R> {
    /// The pipe, until it has been read to its end.
    reader: Option<R>,
    /// What the pipe held so far.
    data: Vec<u8>,
    /// What a read of the pipe reads into.
    chunk: Box<[u8]>,
}

impl<R> Capture<R>
where
    R: AsyncRead + Unpin,
{
    /// A capture of `reader`, which is none for a pipe the child does not have: such a capture is
    /// at the end of its pipe from the start.
    fn new(reader: Option<R>) -> Self {
        let chunk = if reader.is_some() {
            vec![0; CHUNK].into_boxed_slice()
        } else {
            Box::default()
        };

        Self {
            reader,
            data: Vec::new(),
            chunk,
        }
    }

    /// Reads the pipe until it has nothing more at the moment, or has ended, and arranges for
    /// `cx`'s waker to be woken where it has nothing more yet.
    ///
    /// Ready once the pipe has ended, and every call after that finds it so, or has failed.
    fn poll_end(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Some(reader) = &mut self.reader else {
            return Poll::Ready(Ok(()));
        };

        loop {
            match Pin::new(&mut *reader).poll_read(cx, &mut self.chunk) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(0)) => {
                    self.reader = None;
                    return Poll::Ready(Ok(()));
                }
                Poll::Ready(Ok(read)) => self.data.extend_from_slice(&self.chunk[..read]),
                Poll::Ready(Err(error)) if error.kind() == io::ErrorKind::Interrupted => {}
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            }
        }
    }
}

/// How much a read of a pipe takes at most: as much as the reads of `Unblock` do by default.
const CHUNK: usize = 8 * 1024;
