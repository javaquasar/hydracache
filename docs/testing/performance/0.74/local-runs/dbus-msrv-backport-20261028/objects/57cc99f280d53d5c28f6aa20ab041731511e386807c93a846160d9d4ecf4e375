//! Tests of `zruntime::process`: spawning children on a runtime, writing to and reading from the
//! pipes to them, waiting for them, and letting go of them.
//!
//! The commands the tests run are the ones every platform has, run through a shell where they need
//! more than one program: `sh -c` on unix and `cmd /C` on Windows, with the script of each in the
//! shell's own words. What only a shell of unix does, or what only a unix system's pipes and
//! process table make observable, is tested on unix alone. Most of the tests run in both flavours
//! of runtime, through [`in_both_modes`], since a child means the same on either; the ones that
//! need a task are written for a shared runtime.
//!
//! The first tests are of the builder: that it forwards what it is given to the std command, that
//! it reads back whether it kills and whether it reaps a child that is dropped, that it turns a std
//! command into its own, and that what it was not told about a standard stream is decided afresh
//! each time the command runs. Then come the tests of the futures of a command, its output, its
//! status and its failure to spawn, and of the pipes: that a child's input, output and error go
//! through them, that a read that has to wait leaves the thread to the other tasks, that a write
//! that has to wait does, and that the output is read from both pipes at once. After them come the
//! tests of waiting for a child: its status closes the child's input, a status that is given up on
//! can be asked for again, and `try_status` and `kill` agree with it. Then come the pipes handed to
//! another child, what happens to a child that is dropped, whichever way its command said, a child
//! on a task of a shared runtime, and last, the `Debug` of what the module hands out.

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::{io, path::Path, time::Duration};
#[cfg(unix)]
use std::{thread, time::Instant};

use futures::{AsyncRead, AsyncReadExt, AsyncWriteExt, future::join};
use ntest::timeout;
#[cfg(any(target_os = "linux", target_os = "android"))]
use rustix::process::{WaitOptions, waitpid};
#[cfg(unix)]
use rustix::{
    io::Errno,
    process::{Pid, test_kill_process},
};

use crate::{
    Local, LocalRuntime, Runtime, SharedRuntime,
    process::{Child, Command, Stdio},
};

/// Writes the test that follows once per flavour: a module named after it, holding a `local` and
/// a `shared` test that run its body with the mode parameter set to [`Local`](crate::Local) and to
/// [`Shared`](crate::Shared).
///
/// The body builds its runtime with `Runtime::<M>::new()` and drives it with `block_on`, and
/// reaches everything else through the types of the module, which both flavours serve. What it
/// cannot do is spawn a task: the two flavours' `spawn` take different bounds, so a test that
/// spawns is written for one flavour.
macro_rules! in_both_modes {
    ($(#[$attr:meta])* fn $name:ident<$mode:ident>() $body:block) => {
        $(#[$attr])*
        mod $name {
            use super::*;

            #[test]
            #[ntest::timeout(15000)]
            fn local() {
                run::<$crate::Local>();
            }

            #[test]
            #[ntest::timeout(15000)]
            fn shared() {
                run::<$crate::Shared>();
            }

            fn run<$mode>()
            where
                $mode: $crate::Mode,
            $body
        }
    };
}

/// The builder passes what it is given on to the std command, and the std command it hands out
/// tells what that was.
#[test]
#[timeout(15000)]
fn the_builder_forwards_to_the_std_command() {
    let mut command = Command::new("program");
    command
        .arg("one")
        .args(["two", "three"])
        .env("KEPT", "1")
        .envs([("ALSO", "2")])
        .env("DROPPED", "3")
        .env_remove("DROPPED")
        .current_dir("somewhere");

    let std = command.as_std();
    assert_eq!(std.get_program(), "program");
    assert_eq!(std.get_args().collect::<Vec<_>>(), ["one", "two", "three"]);
    assert_eq!(std.get_current_dir(), Some(Path::new("somewhere")));
    let envs = std
        .get_envs()
        .map(|(key, value)| (key.to_owned(), value.map(ToOwned::to_owned)))
        .collect::<Vec<_>>();
    assert!(envs.contains(&("KEPT".into(), Some("1".into()))));
    assert!(envs.contains(&("ALSO".into(), Some("2".into()))));
    // Removed from what the child inherits, which `get_envs` tells by a variable with no value.
    assert!(envs.contains(&("DROPPED".into(), None)));

    command.env_clear();
    assert_eq!(command.as_std().get_envs().count(), 0);
}

/// The getters tell what the setters were given: a command starts out not killing the process of a
/// child that is dropped and collecting its status, as one made from a std command does, and each
/// flag follows its own setter and not the other's.
#[test]
#[timeout(15000)]
fn the_getters_read_back_the_kill_and_reap_settings() {
    let mut command = Command::new("program");
    assert!(!command.get_kill_on_drop());
    assert!(command.get_reap_on_drop());

    command.kill_on_drop(true);
    assert!(command.get_kill_on_drop());
    assert!(command.get_reap_on_drop());

    command.reap_on_drop(false);
    assert!(command.get_kill_on_drop());
    assert!(!command.get_reap_on_drop());

    command.kill_on_drop(false).reap_on_drop(true);
    assert!(!command.get_kill_on_drop());
    assert!(command.get_reap_on_drop());

    let converted = Command::from(std::process::Command::new("program"));
    assert!(!converted.get_kill_on_drop());
    assert!(converted.get_reap_on_drop());
}

in_both_modes! {
    /// What is set on the std command through `as_std_mut` reaches the child: here, a variable of
    /// the environment, which the child prints.
    fn as_std_mut_configures_the_child<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut command = script("echo $ZRUNTIME_TEST", "echo %ZRUNTIME_TEST%");
        command.as_std_mut().env("ZRUNTIME_TEST", "set through std");

        let output = runtime.block_on(command.output(&runtime)).unwrap();

        assert_eq!(text(&output.stdout), "set through std");
    }
}

in_both_modes! {
    /// A command made from a std command counts the standard streams the std command was given as
    /// not configured: the output is captured whatever the std command did with it, as for any
    /// command that was not told about its output.
    fn a_std_command_converts_with_its_streams_not_configured<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut std = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" });
        std.arg(if cfg!(windows) { "/C" } else { "-c" })
            .arg("echo hello")
            .stdout(Stdio::null());
        let mut command = Command::from(std);

        let output = runtime.block_on(command.output(&runtime)).unwrap();

        assert_eq!(text(&output.stdout), "hello");
    }
}

in_both_modes! {
    /// The output holds what the child wrote to its standard output and to its standard error, each
    /// on its own, and the exit code the child ended with.
    fn output_collects_both_streams_and_the_exit_code<M>() {
        let runtime = Runtime::<M>::new().unwrap();

        let output = runtime
            .block_on(
                script(
                    "echo out; echo err >&2; exit 3",
                    "echo out& echo err 1>&2& exit 3",
                )
                .output(&runtime),
            )
            .unwrap();

        assert_eq!(output.status.code(), Some(3));
        assert_eq!(text(&output.stdout), "out");
        assert_eq!(text(&output.stderr), "err");
    }
}

in_both_modes! {
    /// A child that ends well is told so by its status, and has written nothing where it was not
    /// asked to.
    fn output_reports_success<M>() {
        let runtime = Runtime::<M>::new().unwrap();

        let output = runtime
            .block_on(shell("exit 0").output(&runtime))
            .unwrap();

        assert!(output.status.success());
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

in_both_modes! {
    /// A stream the command was told about is as it was told, and one that is not piped is left out
    /// of the output: the standard output set to nothing is not captured, though the output of a
    /// command that was not told about it is.
    fn output_leaves_out_what_is_not_piped<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut command = script("echo out; echo err >&2", "echo out& echo err 1>&2");
        command.stdout(Stdio::null());

        let output = runtime.block_on(command.output(&runtime)).unwrap();

        assert!(output.stdout.is_empty());
        assert_eq!(text(&output.stderr), "err");
    }
}

in_both_modes! {
    /// One command runs any number of times, with any of `spawn`, `status` and `output`, each
    /// finding the streams the command was not told about as it expects: the output captured by
    /// `output` does not stay piped for a `spawn` that follows it, which inherits the streams, and
    /// the inherited ones do not stay so for the `output` after that.
    fn a_command_runs_again_with_fresh_defaults<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut command = shell("exit 0");

        let first = runtime.block_on(command.output(&runtime)).unwrap();
        assert!(first.status.success());

        let mut child = command.spawn(&runtime).unwrap();
        assert!(child.stdin.is_none());
        assert!(child.stdout.is_none());
        assert!(child.stderr.is_none());
        assert!(runtime.block_on(child.status()).unwrap().success());

        assert!(runtime.block_on(command.status(&runtime)).unwrap().success());

        // The streams are piped again for the second `output`: a command that writes shows it.
        let mut echoing = shell("echo hello");
        for _ in 0..2 {
            let output = runtime.block_on(echoing.output(&runtime)).unwrap();
            assert_eq!(text(&output.stdout), "hello");
        }
    }
}

in_both_modes! {
    /// A program that is not there is a spawn that fails with `NotFound`: from `spawn` itself, and,
    /// for `status` and `output`, from the future they hand back, which is how they report it
    /// without having been polled.
    fn a_missing_program_fails_to_spawn<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut command = Command::new("zruntime-no-such-program");

        let spawned = command.spawn(&runtime).unwrap_err();
        let status = runtime.block_on(command.status(&runtime)).unwrap_err();
        let output = runtime.block_on(command.output(&runtime)).unwrap_err();

        assert_eq!(spawned.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(status.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(output.kind(), std::io::ErrorKind::NotFound);
    }
}

in_both_modes! {
    /// What a child is written reaches its input, and what it writes comes back from its output,
    /// the end of its input being a pipe that is dropped.
    fn a_child_writes_back_what_it_reads<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = filter()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn(&runtime)
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        runtime.block_on(async {
            stdin.write_all(b"hello\n").await.unwrap();
            drop(stdin);

            assert_eq!(text(&read_all(stdout).await), "hello");
            assert!(child.status().await.unwrap().success());
        });
    }
}

in_both_modes! {
    /// A read that has to wait for the child leaves the thread to the other tasks: the write that
    /// is due after a delay, from this very thread, is what the read waits for. A read that held
    /// the thread would wait for good, and the test's timeout would fail it.
    fn a_read_that_waits_leaves_the_thread_to_the_other_tasks<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = filter()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn(&runtime)
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let (read, ()) = runtime.block_on(join(read_all(stdout), async {
            runtime.sleep(Duration::from_millis(50)).await;
            stdin.write_all(b"late\n").await.unwrap();
            drop(stdin);
        }));

        assert_eq!(text(&read), "late");
    }
}

in_both_modes! {
    /// Closing the pipe to a child's input, as code that writes to any `AsyncWrite` does once it is
    /// done with it, ends the child's input with the handle still held: the child reads what was
    /// written and then the end, and exits. A write after the close fails, a flush or another close
    /// has nothing to do, and there is no pipe left to hand to another child.
    fn closing_the_input_ends_it<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = filter()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn(&runtime)
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        runtime.block_on(async {
            stdin.write_all(b"alpha\nbravo\n").await.unwrap();
            stdin.close().await.unwrap();

            assert_eq!(text(&read_all(stdout).await), "alpha\nbravo");
            assert!(child.status().await.unwrap().success());

            let written = stdin.write_all(b"charlie\n").await.unwrap_err();
            assert_eq!(written.kind(), io::ErrorKind::BrokenPipe);
            stdin.flush().await.unwrap();
            stdin.close().await.unwrap();
            let handed = stdin.into_stdio().await.unwrap_err();
            assert_eq!(handed.kind(), io::ErrorKind::BrokenPipe);
        });
    }
}

in_both_modes! {
    /// A write that has to wait for room in the pipe leaves the thread to the other tasks, and goes
    /// on once the child has made room: a megabyte, many times what a pipe holds, goes to a child
    /// that writes it back, while the reading of what comes back runs on the same thread.
    #[cfg(unix)]
    fn a_large_input_goes_through_and_comes_back<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let input = pattern(1_000_000);
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn(&runtime)
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let (output, ()) = runtime.block_on(join(read_all(stdout), async {
            stdin.write_all(&input).await.unwrap();
            drop(stdin);
        }));

        assert_eq!(output, input);
    }
}

in_both_modes! {
    /// Vectored writes and reads go through the pipes, the buffers of one written in order.
    #[cfg(unix)]
    fn vectored_io_goes_through_the_pipes<M>() {
        use std::io::{IoSlice, IoSliceMut};

        let runtime = Runtime::<M>::new().unwrap();
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn(&runtime)
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();

        runtime.block_on(async {
            let written = stdin
                .write_vectored(&[IoSlice::new(b"ab"), IoSlice::new(b"cd")])
                .await
                .unwrap();
            assert_eq!(written, 4);
            drop(stdin);

            let (mut first, mut second) = ([0; 1], [0; 8]);
            let read = stdout
                .read_vectored(&mut [IoSliceMut::new(&mut first), IoSliceMut::new(&mut second)])
                .await
                .unwrap();
            // The first read of a pipe may hold less than was written, but what it holds is the
            // front of it, split over the buffers in order.
            assert!(read >= 1);
            assert_eq!(first, *b"a");
            assert_eq!(&second[..read - 1], &b"bcd"[..read - 1]);
        });
    }
}

in_both_modes! {
    /// The output is read from both pipes at once. The child writes more to each of them than a
    /// pipe holds, and exits only once it has written it all, so a reader that finished with one
    /// pipe before it began on the other would wait for the child to exit and the child for the
    /// reader, for good.
    #[cfg(unix)]
    fn output_reads_both_pipes_at_once<M>() {
        let runtime = Runtime::<M>::new().unwrap();

        let output = runtime
            .block_on(
                shell("head -c 200000 /dev/zero; head -c 200000 /dev/zero >&2").output(&runtime),
            )
            .unwrap();

        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 200_000);
        assert_eq!(output.stderr.len(), 200_000);
        assert!(output.stdout.iter().chain(&output.stderr).all(|&byte| byte == 0));
    }
}

in_both_modes! {
    /// The status of a child closes its input first, so that a child that reads its input to the
    /// end can exit: left open, the pipe would have the child wait for it and this wait for the
    /// child, and the test's timeout would fail it.
    fn status_closes_the_input<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = filter()
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn(&runtime)
            .unwrap();
        assert!(child.stdin.is_some());

        let status = runtime.block_on(child.status()).unwrap();

        assert!(status.success());
        assert!(child.stdin.is_none());
    }
}

in_both_modes! {
    /// The output of a child closes its input first as well, and so collects nothing from a child
    /// that only copies what it reads.
    fn output_of_a_child_closes_the_input<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let child = filter()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn(&runtime)
            .unwrap();

        let output = runtime.block_on(child.output()).unwrap();

        assert!(output.status.success());
        assert!(output.stdout.is_empty());
    }
}

in_both_modes! {
    /// `try_status` tells that a child is running while it is, and its status once it has exited,
    /// and a wait for the status after that finds the same one.
    fn try_status_follows_the_child<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = filter()
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn(&runtime)
            .unwrap();

        assert!(child.try_status().unwrap().is_none());
        // `try_status` does not close the input, which is what keeps the child running.
        assert!(child.stdin.is_some());

        drop(child.stdin.take());
        let status = runtime.block_on(child.status()).unwrap();

        assert!(status.success());
        assert_eq!(child.try_status().unwrap(), Some(status));
        assert_eq!(runtime.block_on(child.status()).unwrap(), status);
    }
}

in_both_modes! {
    /// A status that was given up on can be asked for again: the wait that timed out, any number of
    /// times, loses nothing, and the status of the child that was killed meanwhile is the one that
    /// comes out in the end.
    fn a_status_given_up_on_can_be_asked_for_again<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = sleeper().spawn(&runtime).unwrap();

        runtime.block_on(async {
            for _ in 0..3 {
                let waited = runtime
                    .timeout(Duration::from_millis(50), child.status())
                    .await;
                assert!(waited.is_err(), "the child exited by itself");
                assert!(child.try_status().unwrap().is_none());
            }

            child.kill().unwrap();
            let status = child.status().await.unwrap();

            assert!(!status.success());
        });
    }
}

in_both_modes! {
    /// Killing a child ends it, and its status says it did not end well; where the platform says
    /// how, that it was `SIGKILL` that ended it. Killing it again, now that it is gone, is no
    /// error.
    fn kill_ends_a_running_child<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = sleeper().spawn(&runtime).unwrap();

        child.kill().unwrap();
        let status = runtime.block_on(child.status()).unwrap();

        assert!(!status.success());
        #[cfg(unix)]
        assert_eq!(status.signal(), Some(9));
        child.kill().unwrap();
    }
}

in_both_modes! {
    /// A pipe handed to another child connects the two, as a pipeline does: the output of the first
    /// is the input of the second, with no byte through this process. The first writes after a
    /// delay, which a pipe still in non-blocking mode would have the second fail at, with
    /// `EAGAIN`, rather than wait for.
    #[cfg(unix)]
    fn an_output_pipe_becomes_the_input_of_another_child<M>() {
        let runtime = Runtime::<M>::new().unwrap();

        runtime.block_on(async {
            let mut first = shell("sleep 0.2; echo late")
                .stdout(Stdio::piped())
                .spawn(&runtime)
                .unwrap();
            let pipe = first.stdout.take().unwrap().into_stdio().await.unwrap();

            let second = Command::new("cat")
                .stdin(pipe)
                .stdout(Stdio::piped())
                .spawn(&runtime)
                .unwrap();
            let output = second.output().await.unwrap();

            assert!(output.status.success());
            assert_eq!(text(&output.stdout), "late");
            assert!(first.status().await.unwrap().success());
        });
    }
}

in_both_modes! {
    /// The pipe from a child's error is handed over as the pipe from its output is: a child that
    /// writes to its error feeds the input of another.
    #[cfg(unix)]
    fn an_error_pipe_becomes_the_input_of_another_child<M>() {
        let runtime = Runtime::<M>::new().unwrap();

        runtime.block_on(async {
            let mut first = shell("echo oops >&2")
                .stderr(Stdio::piped())
                .spawn(&runtime)
                .unwrap();
            let pipe = first.stderr.take().unwrap().into_stdio().await.unwrap();

            let second = Command::new("cat")
                .stdin(pipe)
                .stdout(Stdio::piped())
                .spawn(&runtime)
                .unwrap();
            let output = second.output().await.unwrap();

            assert_eq!(text(&output.stdout), "oops");
            assert!(first.status().await.unwrap().success());
        });
    }
}

in_both_modes! {
    /// The pipe to a child's input is handed over as the output of another child: the other child
    /// writes into it, and what it writes is what the child reads.
    #[cfg(unix)]
    fn an_input_pipe_becomes_the_output_of_another_child<M>() {
        let runtime = Runtime::<M>::new().unwrap();

        runtime.block_on(async {
            let mut reader = Command::new("cat")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn(&runtime)
                .unwrap();
            let pipe = reader.stdin.take().unwrap().into_stdio().await.unwrap();
            let stdout = reader.stdout.take().unwrap();

            let mut writer = shell("echo hello").stdout(pipe).spawn(&runtime).unwrap();

            assert_eq!(text(&read_all(stdout).await), "hello");
            assert!(writer.status().await.unwrap().success());
            assert!(reader.status().await.unwrap().success());
        });
    }
}

in_both_modes! {
    /// A child killed as it is dropped is gone: the process is told to end, and the status of the
    /// process that ended is collected, which is what takes it out of the process table.
    #[cfg(unix)]
    fn kill_on_drop_kills_the_process<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let child = sleeper().kill_on_drop(true).spawn(&runtime).unwrap();
        let id = child.id();
        assert!(exists(id), "the child is not running");

        drop(child);

        assert!(disappears(id), "the child is still there after being dropped");
    }
}

in_both_modes! {
    /// A child that runs on when it is dropped is waited for on its own, so that it does not
    /// outlive its exit as a zombie: once it has exited, its process is gone from the table.
    #[cfg(unix)]
    fn a_dropped_child_is_reaped_once_it_exits<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let child = shell("sleep 0.3").spawn(&runtime).unwrap();
        let id = child.id();

        drop(child);

        assert!(disappears(id), "the child was never collected");
    }
}

in_both_modes! {
    /// A child dropped after a wait for it was given up on is reaped as well. Where the wait runs
    /// on the pool, the one that was left running and the one the drop starts are both for the same
    /// process, which is collected once; where the runtime watches for the exit, the drop's is the
    /// only one.
    #[cfg(unix)]
    fn a_child_dropped_after_a_given_up_wait_is_reaped<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = shell("sleep 0.5").spawn(&runtime).unwrap();
        let id = child.id();

        let waited = runtime.block_on(runtime.timeout(Duration::from_millis(50), child.status()));
        assert!(waited.is_err(), "the child exited by itself");
        drop(child);

        assert!(disappears(id), "the child was never collected");
    }
}

/// A child that is let go of with `reap_on_drop(false)` is not collected: it stays a zombie until
/// someone collects it, here the test.
///
/// Where the platform tells that a zombie is there. Linux and Android answer a check of a
/// process ID that is a zombie as they do for a running process, which is how the test sees that
/// the status has not been collected.
#[test]
#[timeout(15000)]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn reap_on_drop_off_leaves_the_process_for_someone_else() {
    let runtime = LocalRuntime::new().unwrap();
    let child = shell("exit 0").reap_on_drop(false).spawn(&runtime).unwrap();
    let id = child.id();
    let pid = Pid::from_raw(id as i32).expect("a child's process ID is not zero");

    drop(child);
    // Long enough for something that was going to collect the child to have done so.
    thread::sleep(Duration::from_millis(300));
    assert!(test_kill_process(pid).is_ok(), "the child was collected");

    // Collected here, so that the test leaves no zombie behind.
    waitpid(Some(pid), WaitOptions::empty()).unwrap();
    assert_eq!(test_kill_process(pid), Err(Errno::SRCH));
}

/// The status of a child, and its output, are waited for on a task of a shared runtime: the
/// futures are `Send`, which a task needs of them, and are driven by whichever thread runs the
/// runtime.
#[test]
#[timeout(15000)]
fn a_shared_runtime_waits_for_a_child_on_a_task() {
    let runtime = SharedRuntime::new().unwrap();
    let mut child = script("exit 3", "exit 3").spawn(&runtime).unwrap();
    let waiting = runtime.spawn("a task that waits for a child", async move {
        child.status().await
    });
    let status = runtime.block_on(waiting).unwrap().unwrap();
    assert_eq!(status.code(), Some(3));

    let output = script("echo out; exit 4", "echo out& exit 4").output(&runtime);
    let collecting = runtime.spawn("a task that collects the output of a child", output);
    let output = runtime.block_on(collecting).unwrap().unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(text(&output.stdout), "out");
}

/// A child of a shared runtime goes to another thread with its pipes, and is used there: the
/// pipes are written to and read from by tasks of the runtime that the thread drives.
#[test]
#[timeout(15000)]
fn a_shared_child_goes_to_another_thread() {
    let runtime = SharedRuntime::new().unwrap();
    let mut child = filter()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn(&runtime)
        .unwrap();

    let echoed = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                runtime.block_on(async {
                    let mut stdin = child.stdin.take().unwrap();
                    stdin.write_all(b"moved\n").await.unwrap();
                    drop(stdin);

                    read_all(child.stdout.take().unwrap()).await
                })
            })
            .join()
            .unwrap()
    });

    assert_eq!(text(&echoed), "moved");
}

/// The module's types print: a command as its std command does, and a child with its process ID
/// and its pipes.
#[test]
#[timeout(15000)]
fn the_types_print() {
    let runtime = LocalRuntime::new().unwrap();
    let mut command = filter();
    command.stdin(Stdio::piped()).stdout(Stdio::piped());
    let mut child: Child<Local> = command.spawn(&runtime).unwrap();
    let id = child.id();

    assert!(format!("{command:?}").contains(if cfg!(windows) { "sort" } else { "cat" }));
    let printed = format!("{child:?}");
    assert!(printed.contains("Child"), "{printed}");
    assert!(printed.contains(&id.to_string()), "{printed}");
    assert!(printed.contains("ChildStdin"), "{printed}");
    assert!(printed.contains("ChildStdout"), "{printed}");
    assert_eq!(
        format!("{:?}", child.stderr),
        "None",
        "a pipe that was not asked for is not there"
    );

    drop(child.stdin.take());
    assert!(runtime.block_on(child.status()).unwrap().success());
}

/// A command that runs `script` in the shell of the platform: `sh -c` on unix, `cmd /C` on
/// Windows.
fn shell(script: &str) -> Command {
    let (program, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let mut command = Command::new(program);
    command.arg(flag).arg(script);

    command
}

/// A command that runs `unix` in the shell of a unix system and `windows` in that of Windows,
/// each in its own words.
fn script(unix: &str, windows: &str) -> Command {
    shell(if cfg!(windows) { windows } else { unix })
}

/// A command that copies its input to its output, line by line, and ends with its input: `cat`, and
/// `sort` on Windows, which has no `cat` and writes the lines it reads out in order, which is as
/// they came for one that reads a single line, or lines already sorted.
///
/// `sort` guesses the encoding of its input from what its first read finds, and takes a line of one
/// letter, two bytes with its line feed, for a character of UTF-16, and the whole input with it: a
/// test feeds it words.
fn filter() -> Command {
    Command::new(if cfg!(windows) { "sort" } else { "cat" })
}

/// A command that runs for ten seconds, and writes nothing, so that a test that does not end it
/// by hand times out.
fn sleeper() -> Command {
    if cfg!(windows) {
        let mut command = Command::new("ping");
        command
            .args(["-n", "10", "127.0.0.1"])
            .stdout(Stdio::null());

        command
    } else {
        let mut command = Command::new("sleep");
        command.arg("10");

        command
    }
}

/// What `reader` reads up to its end.
async fn read_all<R>(mut reader: R) -> Vec<u8>
where
    R: AsyncRead + Unpin,
{
    let mut data = Vec::new();
    reader.read_to_end(&mut data).await.unwrap();

    data
}

/// The text of what a child wrote, with the line ends of Windows made the ones of unix and the
/// whitespace around it, which a shell's `echo` adds there, taken off.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .trim()
        .to_owned()
}

/// `len` bytes that tell a change of order, or a loss, from the original.
///
/// They cycle through a prime number of values, so that the pattern does not line up with the size
/// of any buffer on the way.
#[cfg(unix)]
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// Whether a process with the ID `id` is there, a zombie included.
#[cfg(unix)]
fn exists(id: u32) -> bool {
    let pid = Pid::from_raw(id as i32).expect("a child's process ID is not zero");

    test_kill_process(pid) != Err(Errno::SRCH)
}

/// Waits, for a few seconds at most, for the process with the ID `id` to be gone from the process
/// table, and tells whether it went.
///
/// A process that has exited stays there, as a zombie, until its status is collected, so this is
/// also how a test sees that somebody collected it.
#[cfg(unix)]
fn disappears(id: u32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);

    while exists(id) {
        if Instant::now() > deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }

    true
}
